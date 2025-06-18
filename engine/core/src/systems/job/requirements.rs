use serde_json::Value as JsonValue;

/// Returns true if the requirements array is empty or all amounts are zero.
pub fn requirements_are_empty_or_zero(requirements: &[JsonValue]) -> bool {
    requirements.is_empty()
        || requirements
            .iter()
            .all(|req| req.get("amount").and_then(|a| a.as_i64()).unwrap_or(0) == 0)
}

/// Returns true if reserved_resources is missing or is an empty array.
pub fn is_reserved_resources_empty(job: &JsonValue) -> bool {
    job.get("reserved_resources")
        .and_then(|v| v.as_array())
        .map(|a| a.is_empty())
        .unwrap_or(true)
}

/// Returns true if reserved_stockpile is missing or not an integer.
pub fn reserved_stockpile_is_none_or_not_int(job: &JsonValue) -> bool {
    job.get("reserved_stockpile")
        .and_then(|v| v.as_i64())
        .is_none()
}
