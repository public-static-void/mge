//! State transition utilities for the job system.
//!
//! This module provides reusable helpers for common state transition patterns
//! and checks, promoting clarity and reducing repetition in state handlers.

use crate::map::CellKey;
use serde_json::Value as JsonValue;

/// If the agent is at the target cell, transitions the job to "in_progress".
/// Returns `true` if the transition was made.
pub fn transition_if_at_site(
    agent_cell: &CellKey,
    target_cell: &CellKey,
    job: &mut JsonValue,
) -> bool {
    if agent_cell == target_cell {
        job["state"] = serde_json::json!("in_progress");
        true
    } else {
        false
    }
}

/// Returns `true` if all requirements are met (delivered >= needed for each kind).
pub fn are_requirements_met(requirements: &[JsonValue], delivered: &[JsonValue]) -> bool {
    for req in requirements {
        let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let needed = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
        let delivered_amt = delivered
            .iter()
            .find(|r| r.get("kind") == Some(&JsonValue::String(kind.to_string())))
            .and_then(|r| r.get("amount").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        if delivered_amt < needed {
            return false;
        }
    }
    true
}
