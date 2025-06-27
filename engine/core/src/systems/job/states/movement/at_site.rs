//! Handler for the "at_site" job state.

use serde_json::{Value as JsonValue, json};

/// Handles the "at_site" state: transitions the job to "in_progress".
///
/// This state is entered when the agent arrives at the job's target location.
pub fn handle_at_site_state(
    _world: &mut crate::ecs::world::World,
    _eid: u32,
    mut job: JsonValue,
) -> JsonValue {
    job["state"] = json!("in_progress");
    job
}
