use engine_core::ecs::system::System;
use engine_core::systems::job::assign_jobs;
use engine_core::systems::job::{JobBoard, JobSystem};

/// Runs assignment and job system for up to `max_ticks` ticks, breaking early if `pred` returns true.
/// The closure receives a mutable reference to world and should return true to break early.
pub fn run_until<F>(
    world: &mut engine_core::ecs::world::World,
    job_board: &mut JobBoard,
    job_system: &mut JobSystem,
    mut pred: F,
    max_ticks: usize,
) where
    F: FnMut(&mut engine_core::ecs::world::World) -> bool,
{
    for tick in 0..max_ticks {
        job_board.update(world, tick as u64, &[]);
        assign_jobs(world, job_board, tick as u64, &[]);
        job_system.run(world, None);
        if pred(world) {
            break;
        }
    }
}
