use engine_core::ecs::world::World;
use serde_json::json;

/// Test-only helpers for agent manipulation.
pub trait AgentTestHelpers {
    /// Spawns an idle agent and returns its entity ID.
    fn spawn_idle_agent(&mut self) -> u32;
}

impl AgentTestHelpers for World {
    fn spawn_idle_agent(&mut self) -> u32 {
        let agent_eid = self.spawn_entity();
        self.set_component(
            agent_eid,
            "Agent",
            json!({
                "entity_id": agent_eid,
                "state": "idle",
                "current_job": null,
                "job_queue": [],
                "specializations": [],
                "skills": {},
                "preferences": {},
                "stamina": 100,
                "morale": 100,
                "jobs_completed": 0,
                "move_path": [],
                "carried_resources": []
            }),
        )
        .unwrap();
        agent_eid
    }
}
