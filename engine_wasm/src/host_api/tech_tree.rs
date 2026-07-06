//! Tech Tree and Research WASM host API.
//!
//! All functions are registered on the "tech_tree" linker module.
//! Functions return 0 on success, -1 on error (where applicable).
//! String outputs use (ptr, len) pairs.

use crate::host_api::component::{read_wasm_string, write_string_to_wasm};
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::tech_tree;
use serde_json::json;
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Linker};

/// Helper: read a TechProgress component from WasmWorld as a JSON Value.
fn read_tech_progress(world: &WasmWorld, entity: u32) -> serde_json::Value {
    world
        .get_component(entity, "TechProgress")
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| {
            json!({
                "completed": {},
                "queue": [],
                "queue_progress": {},
                "research_points": 0.0,
            })
        })
}

/// Helper: write a TechProgress component to WasmWorld.
fn write_tech_progress(
    world: &mut WasmWorld,
    entity: u32,
    progress: &serde_json::Value,
) -> Result<(), String> {
    world.set_component(
        entity,
        "TechProgress",
        &serde_json::to_string(progress).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Helper: check prerequisites for a tech node against an entity.
fn check_prerequisites(
    world: &WasmWorld,
    entity: u32,
    node: &tech_tree::TechNode,
) -> Result<(), String> {
    for prereq in &node.prerequisites {
        match prereq.prereq_type.as_str() {
            "tech" => {
                let progress = read_tech_progress(world, entity);
                let completed = progress.get("completed").and_then(|c| c.as_object());
                let is_done = completed.map(|m| m.contains_key(&prereq.id)).unwrap_or(false);
                if !is_done {
                    let name = tech_tree::get_tech_node(&prereq.id)
                        .map(|n| n.name.as_str())
                        .unwrap_or(&prereq.id);
                    return Err(format!("Requires tech '{}' ({})", prereq.id, name));
                }
            }
            "skill" => {
                let required_level = prereq.level.unwrap_or(1.0);
                let current_level = world
                    .get_component(entity, "SkillLevels")
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                    .and_then(|sl| {
                        sl.get("skill_levels")
                            .and_then(|slm| slm.as_object())
                            .and_then(|m| m.get(&prereq.id))
                            .and_then(|v| v.as_f64())
                    })
                    .unwrap_or(0.0);
                if current_level < required_level {
                    return Err(format!(
                        "Requires skill '{}' level {} (current: {})",
                        prereq.id, required_level, current_level
                    ));
                }
            }
            other => {
                return Err(format!("Unknown prerequisite type: {other}"));
            }
        }
    }
    Ok(())
}

/// Helper: write a boolean (i32) to WASM memory at the given pointer.
fn write_bool_to_wasm<T>(caller: &mut Caller<T>, ptr: i32, val: bool) {
    let bytes = (if val { 1i32 } else { 0i32 }).to_le_bytes();
    if let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory()) {
        let _ = mem.write(caller, ptr as usize, &bytes);
    }
}

/// Registers the tech tree and research API.
pub fn register_tech_tree_api(linker: &mut Linker<Arc<Mutex<WasmWorld>>>) -> anyhow::Result<()> {
    // get_tech_tree(out_ptr, out_len) -> writes JSON array, returns 0 on success
    linker.func_wrap(
        "tech_tree",
        "get_tech_tree",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let nodes = tech_tree::get_tech_tree();
            let json_str = serde_json::to_string(nodes).unwrap_or_else(|_| "[]".to_string());
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
        },
    )?;

    // get_tech_node(tech_id_ptr, tech_id_len, out_ptr, out_len) -> 0 if found, -1 if not
    linker.func_wrap(
        "tech_tree",
        "get_tech_node",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         tech_id_ptr: i32,
         tech_id_len: i32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let tech_id = match read_wasm_string(&mut caller, tech_id_ptr, tech_id_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            match tech_tree::get_tech_node(&tech_id) {
                Some(node) => {
                    let json_str =
                        serde_json::to_string(node).unwrap_or_else(|_| "{}".to_string());
                    write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
                }
                None => -1,
            }
        },
    )?;

    // get_tech_progress(entity, out_ptr, out_len) -> 0 if found, -1 if not
    linker.func_wrap(
        "tech_tree",
        "get_tech_progress",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json_str = {
                let world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);
                serde_json::to_string(&progress).unwrap_or_else(|_| "{}".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
        },
    )?;

    // get_completed_techs(entity, out_ptr, out_len) -> writes JSON array
    linker.func_wrap(
        "tech_tree",
        "get_completed_techs",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json_str = {
                let world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);
                let techs: Vec<String> = progress
                    .get("completed")
                    .and_then(|c| c.as_object())
                    .map(|m| m.keys().cloned().collect())
                    .unwrap_or_default();
                serde_json::to_string(&techs).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
        },
    )?;

    // is_tech_completed(entity, tech_id_ptr, tech_id_len) -> i32 (1 or 0)
    linker.func_wrap(
        "tech_tree",
        "is_tech_completed",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         tech_id_ptr: i32,
         tech_id_len: i32|
         -> i32 {
            let tech_id = match read_wasm_string(&mut caller, tech_id_ptr, tech_id_len) {
                Ok(s) => s,
                Err(_) => return 0,
            };
            let world = caller.data().lock().unwrap();
            let progress = read_tech_progress(&world, entity);
            let is_done = progress
                .get("completed")
                .and_then(|c| c.as_object())
                .map(|m| m.contains_key(&tech_id))
                .unwrap_or(false);
            if is_done { 1 } else { 0 }
        },
    )?;

    // get_research_queue(entity, out_ptr, out_len) -> writes JSON array
    linker.func_wrap(
        "tech_tree",
        "get_research_queue",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json_str = {
                let world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);
                let queue: Vec<String> = progress
                    .get("queue")
                    .and_then(|q| q.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                serde_json::to_string(&queue).unwrap_or_else(|_| "[]".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
        },
    )?;

    // get_research_queue_progress(entity, out_ptr, out_len) -> writes JSON object
    linker.func_wrap(
        "tech_tree",
        "get_research_queue_progress",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         out_ptr: i32,
         out_len: i32|
         -> i32 {
            let json_str = {
                let world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);
                let default_qp = json!({});
                let qp = progress.get("queue_progress").unwrap_or(&default_qp);
                serde_json::to_string(qp).unwrap_or_else(|_| "{}".to_string())
            };
            write_string_to_wasm(&mut caller, out_ptr, out_len, &json_str) as i32
        },
    )?;

    // research_tech(entity, tech_id_ptr, tech_id_len) -> 0 on success, -1 on error
    linker.func_wrap(
        "tech_tree",
        "research_tech",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         tech_id_ptr: i32,
         tech_id_len: i32|
         -> i32 {
            let tech_id = match read_wasm_string(&mut caller, tech_id_ptr, tech_id_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };

            // Look up tech node
            let node = match tech_tree::get_tech_node(&tech_id) {
                Some(n) => n.clone(),
                None => return -1,
            };

            // Perform the research operation inside a scoped block to drop the lock
            let should_fire_event = {
                let mut world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);

                // Already completed?
                if progress
                    .get("completed")
                    .and_then(|c| c.as_object())
                    .map(|m| m.contains_key(&tech_id))
                    .unwrap_or(false)
                {
                    return -1;
                }

                // Already in queue?
                let in_queue = progress
                    .get("queue")
                    .and_then(|q| q.as_array())
                    .map(|a| a.iter().any(|v| v.as_str() == Some(&tech_id)))
                    .unwrap_or(false);
                if in_queue {
                    return -1;
                }

                // Check prerequisites
                if check_prerequisites(&world, entity, &node).is_err() {
                    return -1;
                }

                // Build updated progress
                let mut queue: Vec<serde_json::Value> = progress
                    .get("queue")
                    .and_then(|q| q.as_array())
                    .cloned()
                    .unwrap_or_default();
                queue.push(json!(tech_id));

                let mut queue_progress = progress
                    .get("queue_progress")
                    .and_then(|qp| qp.as_object())
                    .cloned()
                    .unwrap_or_default();
                queue_progress.insert(tech_id.clone(), json!(0.0));

                let completed = progress
                    .get("completed")
                    .and_then(|c| c.as_object())
                    .cloned()
                    .unwrap_or_default();
                let research_points = progress
                    .get("research_points")
                    .and_then(|rp| rp.as_f64())
                    .unwrap_or(0.0);

                let updated = json!({
                    "completed": serde_json::Value::Object(completed),
                    "queue": serde_json::Value::Array(queue),
                    "queue_progress": serde_json::Value::Object(queue_progress),
                    "research_points": research_points,
                });

                if write_tech_progress(&mut world, entity, &updated).is_err() {
                    return -1;
                }

                true
            };

            // Fire event outside the lock scope to avoid borrow issues
            if should_fire_event {
                let mut world = caller.data().lock().unwrap();
                let event_data = json!({
                    "entity": entity,
                    "tech_id": tech_id,
                    "tech_name": node.name,
                });
                world
                    .send_event("research_started", &event_data.to_string())
                    .ok();
            }

            0
        },
    )?;

    // cancel_research(entity, tech_id_ptr, tech_id_len) -> 0 on success, -1 on error
    linker.func_wrap(
        "tech_tree",
        "cancel_research",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         tech_id_ptr: i32,
         tech_id_len: i32|
         -> i32 {
            let tech_id = match read_wasm_string(&mut caller, tech_id_ptr, tech_id_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };

            let should_fire_event = {
                let mut world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);

                let mut queue: Vec<serde_json::Value> = progress
                    .get("queue")
                    .and_then(|q| q.as_array())
                    .cloned()
                    .unwrap_or_default();
                let pos = queue.iter().position(|v| v.as_str() == Some(&tech_id));
                match pos {
                    Some(idx) => {
                        queue.remove(idx);
                    }
                    None => return -1,
                }

                let mut queue_progress = progress
                    .get("queue_progress")
                    .and_then(|qp| qp.as_object())
                    .cloned()
                    .unwrap_or_default();
                queue_progress.remove(&tech_id);

                let completed = progress
                    .get("completed")
                    .and_then(|c| c.as_object())
                    .cloned()
                    .unwrap_or_default();
                let research_points = progress
                    .get("research_points")
                    .and_then(|rp| rp.as_f64())
                    .unwrap_or(0.0);

                let updated = json!({
                    "completed": serde_json::Value::Object(completed),
                    "queue": serde_json::Value::Array(queue),
                    "queue_progress": serde_json::Value::Object(queue_progress),
                    "research_points": research_points,
                });

                if write_tech_progress(&mut world, entity, &updated).is_err() {
                    return -1;
                }

                true
            };

            if should_fire_event {
                let mut world = caller.data().lock().unwrap();
                let event_data = json!({
                    "entity": entity,
                    "tech_id": tech_id,
                    "tech_name": tech_tree::get_tech_node(&tech_id)
                        .map(|n| n.name.as_str())
                        .unwrap_or(&tech_id),
                });
                world
                    .send_event("research_cancelled", &event_data.to_string())
                    .ok();
            }

            0
        },
    )?;

    // clear_research_queue(entity) -> 0 on success, -1 on error
    linker.func_wrap(
        "tech_tree",
        "clear_research_queue",
        |caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32|
         -> i32 {
            let mut world = caller.data().lock().unwrap();
            let progress = read_tech_progress(&world, entity);
            let completed = progress
                .get("completed")
                .and_then(|c| c.as_object())
                .cloned()
                .unwrap_or_default();
            let research_points = progress
                .get("research_points")
                .and_then(|rp| rp.as_f64())
                .unwrap_or(0.0);

            let updated = json!({
                "completed": serde_json::Value::Object(completed),
                "queue": [],
                "queue_progress": {},
                "research_points": research_points,
            });

            if write_tech_progress(&mut world, entity, &updated).is_err() {
                return -1;
            }
            0
        },
    )?;

    // can_research_tech(entity, tech_id_ptr, tech_id_len, out_bool_ptr, out_reason_ptr, out_reason_len)
    // -> 0 on success, -1 on error
    linker.func_wrap(
        "tech_tree",
        "can_research_tech",
        |mut caller: Caller<'_, Arc<Mutex<WasmWorld>>>,
         entity: u32,
         tech_id_ptr: i32,
         tech_id_len: i32,
         out_bool_ptr: i32,
         out_reason_ptr: i32,
         out_reason_len: i32|
         -> i32 {
            let tech_id = match read_wasm_string(&mut caller, tech_id_ptr, tech_id_len) {
                Ok(s) => s,
                Err(_) => return -1,
            };

            // Look up tech node (static lookup, doesn't need lock)
            let node = match tech_tree::get_tech_node(&tech_id) {
                Some(n) => n,
                None => {
                    let reason = format!("Unknown tech '{}'", tech_id);
                    write_bool_to_wasm(&mut caller, out_bool_ptr, false);
                    write_string_to_wasm(&mut caller, out_reason_ptr, out_reason_len, &reason);
                    return 0;
                }
            };

            // Compute reason and can_research
            let (can_research, reason) = {
                let world = caller.data().lock().unwrap();
                let progress = read_tech_progress(&world, entity);

                // Already completed?
                if progress
                    .get("completed")
                    .and_then(|c| c.as_object())
                    .map(|m| m.contains_key(&tech_id))
                    .unwrap_or(false)
                {
                    (false, format!("Tech '{}' already completed", tech_id))
                }
                // Already in queue?
                else if progress
                    .get("queue")
                    .and_then(|q| q.as_array())
                    .map(|a| a.iter().any(|v| v.as_str() == Some(&tech_id)))
                    .unwrap_or(false)
                {
                    (false, format!("Tech '{}' already in research queue", tech_id))
                }
                // Check prerequisites
                else {
                    match check_prerequisites(&world, entity, node) {
                        Ok(_) => (true, String::new()),
                        Err(e) => (false, e),
                    }
                }
            };

            // Write results (lock is dropped by now)
            write_bool_to_wasm(&mut caller, out_bool_ptr, can_research);
            write_string_to_wasm(&mut caller, out_reason_ptr, out_reason_len, &reason);

            0
        },
    )?;

    Ok(())
}
