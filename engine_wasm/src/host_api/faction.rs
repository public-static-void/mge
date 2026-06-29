use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the faction/reputation API (set_faction, get_faction, modify_reputation, get_reputation).
pub fn register_faction_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "faction",
        "set_faction",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         faction_id_ptr: i32,
         faction_id_len: i32,
         role_ptr: i32,
         role_len: i32| {
            let faction_id = read_wasm_string(&mut caller, faction_id_ptr, faction_id_len)
                .expect("Failed to read faction_id from WASM memory");
            let role = read_wasm_string(&mut caller, role_ptr, role_len)
                .expect("Failed to read role from WASM memory");
            let mut world = caller.data().lock().unwrap();
            let joined_tick = world.turn;
            world
                .set_component(
                    entity,
                    "Faction",
                    &json!({
                        "faction_id": faction_id,
                        "role": role,
                        "joined_tick": joined_tick,
                    })
                    .to_string(),
                )
                .expect("Failed to set Faction component");
        },
    )?;

    linker.func_wrap(
        "faction",
        "get_faction",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let result = {
                let world = caller.data().lock().unwrap();
                world
                    .get_component(entity, "Faction")
                    .and_then(|json_str| {
                        serde_json::from_str::<serde_json::Value>(&json_str).ok()
                    })
                    .and_then(|v| {
                        v.get("faction_id")
                            .and_then(|id| id.as_str().map(|s| s.to_string()))
                    })
            };
            match result {
                Some(faction_id) => {
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &faction_id) as i32
                }
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "faction",
        "modify_reputation",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         faction_id_ptr: i32,
         faction_id_len: i32,
         delta: i64| {
            let faction_id = read_wasm_string(&mut caller, faction_id_ptr, faction_id_len)
                .expect("Failed to read faction_id from WASM memory");
            let mut world = caller.data().lock().unwrap();

            // Read current reputation or default
            let old_value = world
                .get_component(entity, "Reputation")
                .and_then(|json_str| {
                    serde_json::from_str::<serde_json::Value>(&json_str).ok()
                })
                .and_then(|v| {
                    v.get("values")
                        .and_then(|vals| vals.get(&faction_id).and_then(|s| s.as_i64()))
                })
                .unwrap_or(0);

            let new_value = (old_value + delta).clamp(-100, 100);

            // Build and set the Reputation component
            let decay_rate = world
                .get_component(entity, "Reputation")
                .and_then(|json_str| {
                    serde_json::from_str::<serde_json::Value>(&json_str).ok()
                })
                .and_then(|v| v.get("decay_rate").cloned())
                .unwrap_or(json!(0.0));

            // Get existing values map or create empty
            let mut values_map = world
                .get_component(entity, "Reputation")
                .and_then(|json_str| {
                    serde_json::from_str::<serde_json::Value>(&json_str).ok()
                })
                .and_then(|v| {
                    v.get("values")
                        .and_then(|vals| vals.as_object())
                        .map(|obj| {
                            let mut map = serde_json::Map::new();
                            for (k, v) in obj {
                                map.insert(k.clone(), v.clone());
                            }
                            map
                        })
                })
                .unwrap_or_default();

            values_map.insert(faction_id.clone(), json!(new_value));

            world
                .set_component(
                    entity,
                    "Reputation",
                    &json!({
                        "values": serde_json::Value::Object(values_map),
                        "decay_rate": decay_rate,
                    })
                    .to_string(),
                )
                .expect("Failed to set Reputation component");

            // Emit event
            let event_data = json!({
                "entity": entity,
                "faction": faction_id,
                "old": old_value,
                "new": new_value,
                "delta": delta,
            });
            world
                .send_event("reputation_changed", &event_data.to_string())
                .ok();
        },
    )?;

    linker.func_wrap(
        "faction",
        "get_reputation",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         faction_id_ptr: i32,
         faction_id_len: i32|
         -> i64 {
            let faction_id = read_wasm_string(&mut caller, faction_id_ptr, faction_id_len)
                .expect("Failed to read faction_id from WASM memory");
            let world = caller.data().lock().unwrap();
            world
                .get_component(entity, "Reputation")
                .and_then(|json_str| {
                    serde_json::from_str::<serde_json::Value>(&json_str).ok()
                })
                .and_then(|v| {
                    v.get("values")
                        .and_then(|vals| vals.get(&faction_id))
                        .and_then(|s| s.as_i64())
                })
                .unwrap_or(0)
        },
    )?;

    Ok(())
}
