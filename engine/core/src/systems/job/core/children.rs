use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Processes all child jobs of a parent job, updating their states and propagating cancellation if needed.
/// Returns a tuple: (updated children array, all_children_complete: bool)
pub fn process_job_children(
    world: &mut World,
    lua: Option<&mlua::Lua>,
    eid: u32,
    children_val: &mut JsonValue,
    is_cancelled: bool,
) -> (JsonValue, bool) {
    let mut children = std::mem::take(children_val)
        .as_array_mut()
        .map(std::mem::take)
        .unwrap_or_default();
    let mut all_children_complete = true;
    for child in &mut children {
        let processed =
            crate::systems::job::system::process::process_job(world, lua, eid, child.take());
        if is_cancelled {
            *child = processed;
            child["state"] = JsonValue::from("cancelled");
        } else {
            *child = processed;
        }
        if child.get("state").and_then(|v| v.as_str()) != Some("complete") {
            all_children_complete = false;
        }
    }
    let children_is_empty = children.is_empty();
    (
        JsonValue::Array(children),
        !is_cancelled && !children_is_empty && all_children_complete,
    )
}
