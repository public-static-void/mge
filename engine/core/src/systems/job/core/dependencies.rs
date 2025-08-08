use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Evaluates whether all dependencies are satisfied for a job.
/// Supports AND/OR/NOT, world_state, and entity_state dependencies.
/// Jobs with no dependencies are always considered satisfied.
pub fn dependencies_satisfied(world: &World, job: &JsonValue) -> bool {
    match job.get("dependencies") {
        None | Some(JsonValue::Null) => true,
        Some(dep) => evaluate_dependency_expr(world, dep, false),
    }
}

/// Returns true if a job is in a terminal state (complete, failed, or cancelled).
fn is_forbidden_state(job: &serde_json::Value) -> bool {
    matches!(
        job.get("state").and_then(|v| v.as_str()),
        Some("complete") | Some("failed") | Some("cancelled")
    )
}

/// Recursively evaluate a dependency expression.
/// `in_not` is true if we're inside a NOT clause.
fn evaluate_dependency_expr(world: &World, dep: &JsonValue, in_not: bool) -> bool {
    if dep.is_string() {
        let dep_eid = dep.as_str().unwrap();
        if let Ok(eid) = dep_eid.parse::<u32>() {
            if let Some(dep_job) = world.get_component(eid, "Job") {
                if in_not {
                    // For NOT, forbidden state blocks satisfaction
                    return !is_forbidden_state(dep_job);
                } else {
                    // For direct dep, only "complete" is satisfied
                    return dep_job
                        .get("state")
                        .map(|s| s == "complete")
                        .unwrap_or(false);
                }
            } else {
                // If job doesn't exist, treat as satisfied if in NOT, else not satisfied
                return in_not;
            }
        }
        // Not a valid entity: treat as satisfied if in NOT, else not satisfied
        return in_not;
    }
    if dep.is_array() {
        return dep
            .as_array()
            .unwrap()
            .iter()
            .all(|d| evaluate_dependency_expr(world, d, in_not));
    }
    if dep.is_object() {
        let obj = dep.as_object().unwrap();
        if let Some(all_of) = obj.get("all_of") {
            return all_of
                .as_array()
                .unwrap()
                .iter()
                .all(|d| evaluate_dependency_expr(world, d, in_not));
        }
        if let Some(any_of) = obj.get("any_of") {
            return any_of
                .as_array()
                .unwrap()
                .iter()
                .any(|d| evaluate_dependency_expr(world, d, in_not));
        }
        if let Some(not) = obj.get("not") {
            // NOT: satisfied if none inside are in a forbidden state
            return !not.as_array().unwrap().iter().any(|d| {
                if d.is_string() {
                    let dep_eid = d.as_str().unwrap();
                    if let Ok(eid) = dep_eid.parse::<u32>() {
                        if let Some(dep_job) = world.get_component(eid, "Job") {
                            return is_forbidden_state(dep_job);
                        }
                        // If job doesn't exist, NOT is satisfied for this dep
                        return false;
                    }
                }
                // For complex expressions, fallback to old logic
                evaluate_dependency_expr(world, d, true)
            });
        }
        if let Some(ws) = obj.get("world_state") {
            return evaluate_world_state(world, ws);
        }
        if let Some(es) = obj.get("entity_state") {
            return evaluate_entity_state(world, es);
        }
    }
    false
}

/// Evaluates a world state dependency.
pub fn evaluate_world_state(world: &World, ws: &JsonValue) -> bool {
    let resource = ws.get("resource").and_then(|v| v.as_str());
    if let Some(res) = resource {
        let value = world.get_global_resource_amount(res);
        let gte = ws.get("gte").and_then(|v| v.as_f64());
        let lte = ws.get("lte").and_then(|v| v.as_f64());
        if let Some(gte) = gte {
            if value < gte {
                return false;
            }
        }
        if let Some(lte) = lte {
            if value > lte {
                return false;
            }
        }
        return true;
    }
    false
}

/// Evaluates an entity state dependency.
pub fn evaluate_entity_state(world: &World, es: &JsonValue) -> bool {
    let entity = es.get("entity").and_then(|v| v.as_u64()).map(|v| v as u32);
    let component = es.get("component").and_then(|v| v.as_str());
    let field = es.get("field").and_then(|v| v.as_str());
    if let (Some(eid), Some(comp), Some(field)) = (entity, component, field) {
        if let Some(comp_val) = world.get_component(eid, comp) {
            let value = comp_val
                .get(field)
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::NAN);
            let gte = es.get("gte").and_then(|v| v.as_f64());
            let lte = es.get("lte").and_then(|v| v.as_f64());
            if let Some(gte) = gte {
                if value < gte {
                    return false;
                }
            }
            if let Some(lte) = lte {
                if value > lte {
                    return false;
                }
            }
            return true;
        }
    }
    false
}

/// Returns Some("failed") or Some("cancelled") if any dependency has failed or been cancelled, otherwise None.
pub fn dependency_failure_state(world: &World, job: &JsonValue) -> Option<&'static str> {
    match job.get("dependencies") {
        None | Some(JsonValue::Null) => None,
        Some(dep) => find_failure_state(world, dep),
    }
}

/// Recursively checks for dependency failure/cancelled state.
/// NOTE: NOT clauses do NOT propagate failure/cancelled state from their referenced jobs.
fn find_failure_state(world: &World, dep: &serde_json::Value) -> Option<&'static str> {
    if dep.is_string() {
        let dep_eid = dep.as_str().unwrap();
        if let Ok(eid) = dep_eid.parse::<u32>() {
            // Only propagate failure if entity exists
            if world.entity_exists(eid) {
                if let Some(dep_job) = world.get_component(eid, "Job") {
                    let state = dep_job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                    if state == "failed" {
                        return Some("failed");
                    }
                    if state == "cancelled" {
                        return Some("cancelled");
                    }
                }
            }
        }
        // If job doesn't exist, do NOT propagate failure
        return None;
    }
    if dep.is_array() {
        for d in dep.as_array().unwrap() {
            if let Some(state) = find_failure_state(world, d) {
                return Some(state);
            }
        }
        return None;
    }
    if dep.is_object() {
        let obj = dep.as_object().unwrap();
        // NOT: do NOT propagate failure/cancelled state from referenced jobs
        if obj.get("not").is_some() {
            return None;
        }
        for key in &["all_of", "any_of"] {
            if let Some(arr) = obj.get(*key) {
                for d in arr.as_array().unwrap() {
                    if let Some(state) = find_failure_state(world, d) {
                        return Some(state);
                    }
                }
            }
        }
    }
    None
}
