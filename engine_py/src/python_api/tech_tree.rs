use super::world::PyWorld;
use engine_core::tech_tree;
use pyo3::prelude::*;
use serde_json::Value as JsonValue;

/// API for the tech tree and research system
pub trait TechTreeApi {
    /// Returns all tech tree nodes as a JSON value
    fn get_tech_tree(&self) -> JsonValue;
    /// Returns a specific tech node by ID, or JsonValue::Null
    fn get_tech_node(&self, tech_id: &str) -> JsonValue;
    /// Returns the TechProgress component for an entity, or JsonValue::Null
    fn get_tech_progress(&self, entity: u32) -> Option<JsonValue>;
    /// Returns a list of completed tech IDs for an entity
    fn get_completed_techs(&self, entity: u32) -> Vec<String>;
    /// Checks if a tech is completed for an entity
    fn is_tech_completed(&self, entity: u32, tech_id: &str) -> bool;
    /// Returns the current research queue for an entity
    fn get_research_queue(&self, entity: u32) -> Vec<String>;
    /// Returns the queue progress map as JSON
    fn get_research_queue_progress(&self, entity: u32) -> JsonValue;
    /// Adds a tech to the research queue
    fn research_tech(&self, entity: u32, tech_id: &str) -> PyResult<()>;
    /// Removes a tech from the research queue
    fn cancel_research(&self, entity: u32, tech_id: &str) -> PyResult<()>;
    /// Empties the research queue
    fn clear_research_queue(&self, entity: u32) -> PyResult<()>;
    /// Checks if an entity can research a tech, returns (can_research, reason)
    fn can_research_tech(&self, entity: u32, tech_id: &str) -> (bool, String);
}

impl TechTreeApi for PyWorld {
    fn get_tech_tree(&self) -> JsonValue {
        let nodes = tech_tree::get_tech_tree();
        serde_json::to_value(nodes).unwrap_or_default()
    }

    fn get_tech_node(&self, tech_id: &str) -> JsonValue {
        tech_tree::get_tech_node(tech_id)
            .and_then(|node| serde_json::to_value(node).ok())
            .unwrap_or(JsonValue::Null)
    }

    fn get_tech_progress(&self, entity: u32) -> Option<JsonValue> {
        let world = self.inner.borrow();
        tech_tree::get_tech_progress(&world, entity)
    }

    fn get_completed_techs(&self, entity: u32) -> Vec<String> {
        let world = self.inner.borrow();
        tech_tree::get_completed_techs(&world, entity)
    }

    fn is_tech_completed(&self, entity: u32, tech_id: &str) -> bool {
        let world = self.inner.borrow();
        tech_tree::is_tech_completed(&world, entity, tech_id)
    }

    fn get_research_queue(&self, entity: u32) -> Vec<String> {
        let world = self.inner.borrow();
        tech_tree::get_research_queue(&world, entity)
    }

    fn get_research_queue_progress(&self, entity: u32) -> JsonValue {
        let world = self.inner.borrow();
        tech_tree::get_research_queue_progress(&world, entity)
    }

    fn research_tech(&self, entity: u32, tech_id: &str) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        tech_tree::research_tech(&mut world, entity, tech_id)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn cancel_research(&self, entity: u32, tech_id: &str) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        tech_tree::cancel_research(&mut world, entity, tech_id)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn clear_research_queue(&self, entity: u32) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        tech_tree::clear_research_queue(&mut world, entity)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn can_research_tech(&self, entity: u32, tech_id: &str) -> (bool, String) {
        let world = self.inner.borrow();
        match tech_tree::can_research_tech(&world, entity, tech_id) {
            Ok(true) => (true, String::new()),
            Ok(false) => (false, "Unknown reason".to_string()),
            Err(reason) => (false, reason),
        }
    }
}
