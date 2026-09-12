//! Weather API: get_weather, set_weather, get_weather_visibility_modifier.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Registers the weather API (get_weather, set_weather, get_weather_visibility_modifier).
pub fn register_weather_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    linker.func_wrap(
        "weather",
        "get_weather",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>, out_ptr: i32, out_len: i32| -> i32 {
            let weather = {
                let world = caller.data().lock().unwrap();
                world.get_weather()
            };
            let json = serde_json::json!({
                "condition": weather.condition.as_str(),
                "intensity": weather.intensity,
                "duration_remaining": weather.duration_remaining,
            });
            let json_str = serde_json::to_string(&json).unwrap_or_else(|_| "{}".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
        },
    )?;

    linker.func_wrap(
        "weather",
        "set_weather",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         condition_ptr: i32,
         condition_len: i32,
         intensity: f64,
         duration: u32|
         -> i32 {
            let condition = read_wasm_string(&mut caller, condition_ptr, condition_len)
                .expect("Failed to read weather condition from WASM memory");
            let mut world = caller.data().lock().unwrap();
            let old_condition = world.weather.condition;
            world.set_weather(&condition, intensity, duration);
            let new_condition = world.weather.condition;
            let new_intensity = world.weather.intensity;
            // Emit a "weather_changed" event like natural transitions (OQ4).
            let payload = serde_json::json!({
                "old_condition": old_condition.as_str(),
                "new_condition": new_condition.as_str(),
                "intensity": new_intensity,
            });
            let _ = world.send_event(
                "weather_changed",
                &serde_json::to_string(&payload).unwrap_or_default(),
            );
            0
        },
    )?;

    linker.func_wrap(
        "weather",
        "get_weather_visibility_modifier",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>| -> f64 {
            let world = caller.data().lock().unwrap();
            world.get_weather_visibility_modifier()
        },
    )?;

    Ok(())
}
