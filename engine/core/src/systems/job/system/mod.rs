//! Job system core: orchestrates job processing, effects, and events.

pub mod effects;
pub mod events;
pub mod orchestrator;
pub mod process;

pub use effects::*;
pub use events::*;
pub use orchestrator::{cleanup_agent_on_job_state, should_spawn_conditional_child};
pub use process::*;

use crate::ecs::system::System;
use crate::ecs::world::World;

#[derive(Default)]
pub struct JobSystem;

impl JobSystem {
    pub fn new() -> Self {
        JobSystem
    }
}

impl System for JobSystem {
    fn name(&self) -> &'static str {
        "JobSystem"
    }

    fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>) {
        crate::systems::job::system::orchestrator::run_job_system(world, lua);
    }
}
