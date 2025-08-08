use crate::python_api::world::PyWorld;
use engine_core::ecs::system::System;
use engine_core::systems::job::reservation::ResourceReservationSystem;
use pyo3::prelude::*;

/// Get the reserved resources for a job by entity ID.
/// Returns a list of dicts or None.
pub fn get_job_resource_reservations(
    pyworld: &PyWorld,
    entity_id: u32,
    py: Python,
) -> PyResult<Option<PyObject>> {
    let world = pyworld.inner.borrow();
    if let Some(job) = world.get_component(entity_id, "Job")
        && let Some(reserved) = job.get("reserved_resources")
            && let Some(arr) = reserved.as_array() {
                if arr.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(serde_pyobject::to_pyobject(py, reserved)?.into()));
            }
    Ok(None)
}

/// Reserve job resources for job entity by ID.
pub fn reserve_job_resources(pyworld: &PyWorld, entity_id: u32) -> PyResult<bool> {
    let world = pyworld.inner.borrow_mut();
    let system = ResourceReservationSystem;
    let status = system.check_reservation_status(&world, entity_id);
    Ok(matches!(
        status,
        engine_core::systems::job::reservation::ResourceReservationStatus::Reserved
    ))
}

/// Release job resource reservations for job entity by ID.
pub fn release_job_resource_reservations(pyworld: &PyWorld, entity_id: u32) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    let system = ResourceReservationSystem;
    system.release_reservation(&mut world, entity_id);
    Ok(())
}

/// Run the resource reservation system explicitly.
pub fn run_resource_reservation_system(pyworld: &PyWorld) -> PyResult<()> {
    let mut world = pyworld.inner.borrow_mut();
    let mut system = ResourceReservationSystem::new();
    system.run(&mut world, None);
    Ok(())
}
