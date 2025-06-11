use super::Map;
use super::cell_key::CellKey;
use super::hex::HexGridMap;
use super::region::RegionMap;
use super::square::SquareGridMap;
use crate::map::topology::MapTopology;
use serde_json::Value;
use std::fs;

/// Validate map JSON against the schema before parsing.
pub fn validate_map_schema(map_json: &Value) -> Result<(), String> {
    let schema_path = format!("{}/../assets/schemas/map.json", env!("CARGO_MANIFEST_DIR"));
    let schema_str = fs::read_to_string(&schema_path)
        .map_err(|e| format!("Failed to read map schema at {}: {}", schema_path, e))?;
    let schema_json: Value = serde_json::from_str(&schema_str)
        .map_err(|e| format!("Failed to parse map schema: {}", e))?;
    let validator = jsonschema::validator_for(&schema_json)
        .map_err(|e| format!("Failed to compile map schema: {}", e))?;
    let errors: Vec<String> = validator
        .iter_errors(map_json)
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Map schema validation failed: {}",
            errors.join("; ")
        ))
    }
}

/// Convert a JSON map to a Map object
pub fn map_from_json(value: &Value) -> Option<Map> {
    if let Err(e) = validate_map_schema(value) {
        eprintln!("Map schema validation failed: {}", e);
        return None;
    }
    let topology = value.get("topology")?.as_str()?;
    match topology {
        "square" => {
            let mut map = SquareGridMap::new();
            for cell in value.get("cells")?.as_array()? {
                let x = cell.get("x")?.as_i64()? as i32;
                let y = cell.get("y")?.as_i64()? as i32;
                let z = cell.get("z")?.as_i64()? as i32;
                map.add_cell(x, y, z);
                if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                    for n in neighs {
                        let nx = n.get("x")?.as_i64()? as i32;
                        let ny = n.get("y")?.as_i64()? as i32;
                        let nz = n.get("z")?.as_i64()? as i32;
                        map.add_neighbor((x, y, z), (nx, ny, nz));
                    }
                }
                if let Some(meta) = cell.get("metadata") {
                    let key = CellKey::Square { x, y, z };
                    map.set_cell_metadata(&key, meta.clone());
                }
            }
            Some(Map::new(Box::new(map)))
        }
        "hex" => {
            let mut map = HexGridMap::new();
            for cell in value.get("cells")?.as_array()? {
                let q = cell.get("q")?.as_i64()? as i32;
                let r = cell.get("r")?.as_i64()? as i32;
                let z = cell.get("z")?.as_i64()? as i32;
                map.add_cell(q, r, z);
                if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                    for n in neighs {
                        let nq = n.get("q")?.as_i64()? as i32;
                        let nr = n.get("r")?.as_i64()? as i32;
                        let nz = n.get("z")?.as_i64()? as i32;
                        map.add_neighbor((q, r, z), (nq, nr, nz));
                    }
                }
                if let Some(meta) = cell.get("metadata") {
                    let key = CellKey::Hex { q, r, z };
                    map.set_cell_metadata(&key, meta.clone());
                }
            }
            Some(Map::new(Box::new(map)))
        }
        "region" => {
            let mut map = RegionMap::new();
            for cell in value.get("cells")?.as_array()? {
                let id = cell.get("id")?.as_str()?.to_string();
                map.add_cell(&id);
                if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                    for n in neighs {
                        let nid = n.as_str()?.to_string();
                        map.add_neighbor(&id, &nid);
                    }
                }
                if let Some(meta) = cell.get("metadata") {
                    let key = CellKey::Region { id: id.clone() };
                    map.set_cell_metadata(&key, meta.clone());
                }
            }
            Some(Map::new(Box::new(map)))
        }
        _ => None,
    }
}
