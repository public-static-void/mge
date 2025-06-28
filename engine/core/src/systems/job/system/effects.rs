//! Effect application and rollback logic for the job system.

use crate::ecs::world::World;
use crate::systems::job::core::dependencies::{evaluate_entity_state, evaluate_world_state};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use std::sync::Mutex;

pub fn process_job_effects(
    world: &mut World,
    job_id: u32,
    job_type: &str,
    job: &mut serde_json::Map<String, JsonValue>,
    on_cancel: bool,
) {
    let empty = Vec::new();
    let effects: Vec<_> = world
        .job_types
        .get_data(job_type)
        .map(|jt| jt.effects.clone())
        .unwrap_or_else(|| empty.clone());

    let effect_registry = world.effect_processor_registry.as_ref().unwrap().clone();

    // Extract applied_effects BEFORE mutable borrow
    let mut applied_effect_indices = job
        .get("applied_effects")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(Vec::new);

    fn process_effect_and_chains(
        effect_registry: &Arc<
            Mutex<
                crate::systems::job::registry::effect_processor_registry::EffectProcessorRegistry,
            >,
        >,
        world: &mut World,
        job_id: u32,
        effect: &JsonValue,
    ) {
        effect_registry.lock().unwrap().process_effects(
            world,
            job_id,
            std::slice::from_ref(effect),
        );
        if let Some(chained) = effect.get("effects").and_then(|v| v.as_array()) {
            for chained_effect in chained {
                process_effect_and_chains(effect_registry, world, job_id, chained_effect);
            }
        }
    }

    if on_cancel {
        for idx in &applied_effect_indices {
            if let Some(effect_idx) = idx.as_u64() {
                if let Some(effect) = effects.get(effect_idx as usize) {
                    let effect_value = serde_json::to_value(effect.clone()).unwrap();
                    effect_registry.lock().unwrap().rollback_effects(
                        world,
                        job_id,
                        &[effect_value],
                    );
                }
            }
        }
        applied_effect_indices.clear();
    } else {
        // --- Incremental: Apply only one effect per tick ---
        let mut next_effect_idx = None;
        for (idx, effect) in effects.iter().enumerate() {
            let already_applied = applied_effect_indices
                .iter()
                .any(|v| v.as_u64() == Some(idx as u64));
            if already_applied {
                continue;
            }
            let should_apply = match effect.get("condition") {
                None => true,
                Some(cond) => {
                    if let Some(ws) = cond.get("world_state") {
                        evaluate_world_state(world, ws)
                    } else if let Some(es) = cond.get("entity_state") {
                        evaluate_entity_state(world, es)
                    } else {
                        false
                    }
                }
            };
            if !should_apply {
                continue;
            }
            next_effect_idx = Some(idx);
            break;
        }
        if let Some(idx) = next_effect_idx {
            let effect = &effects[idx];
            let effect_value = serde_json::to_value(effect.clone()).unwrap();
            process_effect_and_chains(&effect_registry, world, job_id, &effect_value);
            applied_effect_indices.push(JsonValue::from(idx as u64));
        }
    }

    job.insert(
        "applied_effects".to_string(),
        JsonValue::Array(applied_effect_indices),
    );
}
