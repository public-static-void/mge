use super::Map;
use super::cell_key::CellKey;
use super::hex::HexGridMap;
use super::province::ProvinceMap;
use super::square::SquareGridMap;
use crate::map::topology::MapTopology;
use serde_json::Value;
use std::fs;

/// Validate map JSON against the schema before parsing.
pub fn validate_map_schema(map_json: &Value) -> Result<(), String> {
    let schema_path = format!("{}/../assets/schemas/map.json", env!("CARGO_MANIFEST_DIR"));
    let schema_str = fs::read_to_string(&schema_path)
        .map_err(|e| format!("Failed to read map schema at {schema_path}: {e}"))?;
    let schema_json: Value = serde_json::from_str(&schema_str)
        .map_err(|e| format!("Failed to parse map schema: {e}"))?;
    let validator = jsonschema::validator_for(&schema_json)
        .map_err(|e| format!("Failed to compile map schema: {e}"))?;
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
        eprintln!("Map schema validation failed: {e}");
        return None;
    }
    let topology = value.get("topology")?.as_str()?;
    match topology {
        "square" => {
            let mut map = SquareGridMap::new();
            // Collect all cells to a Vec for neighbor inference later
            let cells = value.get("cells")?.as_array()?;
            // First add all cells
            for cell in cells {
                let x = cell.get("x")?.as_i64()? as i32;
                let y = cell.get("y")?.as_i64()? as i32;
                let z = cell.get("z")?.as_i64()? as i32;
                map.add_cell(x, y, z);
            }
            // Then add neighbors (explicit or inferred)
            for cell in cells {
                let x = cell.get("x")?.as_i64()? as i32;
                let y = cell.get("y")?.as_i64()? as i32;
                let z = cell.get("z")?.as_i64()? as i32;

                if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                    // Explicit neighbors
                    for n in neighs {
                        let nx = n.get("x")?.as_i64()? as i32;
                        let ny = n.get("y")?.as_i64()? as i32;
                        let nz = n.get("z")?.as_i64()? as i32;
                        map.add_neighbor((x, y, z), (nx, ny, nz));
                    }
                } else {
                    // No explicit neighbors: infer 4-way adjacency
                    let candidate_neighbors =
                        [(x + 1, y, z), (x - 1, y, z), (x, y + 1, z), (x, y - 1, z)];
                    for &(nx, ny, nz) in &candidate_neighbors {
                        if map.contains(&CellKey::Square {
                            x: nx,
                            y: ny,
                            z: nz,
                        }) {
                            map.add_neighbor((x, y, z), (nx, ny, nz));
                        }
                    }
                }

                // Set metadata if present
                if let Some(meta) = cell.get("metadata") {
                    let key = CellKey::Square { x, y, z };
                    map.set_cell_metadata(&key, meta.clone());
                }
            }
            Some(Map::new(Box::new(map)))
        }

        // For hex topology, similar neighbor inference can be added if desired
        "hex" => {
            let mut map = HexGridMap::new();
            let cells = value.get("cells")?.as_array()?;

            // First add all cells
            for cell in cells {
                let q = cell.get("q")?.as_i64()? as i32;
                let r = cell.get("r")?.as_i64()? as i32;
                let z = cell.get("z")?.as_i64()? as i32;
                map.add_cell(q, r, z);
            }

            // Add neighbors (explicit or inferred)
            for cell in cells {
                let q = cell.get("q")?.as_i64()? as i32;
                let r = cell.get("r")?.as_i64()? as i32;
                let z = cell.get("z")?.as_i64()? as i32;

                if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                    for n in neighs {
                        let nq = n.get("q")?.as_i64()? as i32;
                        let nr = n.get("r")?.as_i64()? as i32;
                        let nz = n.get("z")?.as_i64()? as i32;
                        map.add_neighbor((q, r, z), (nq, nr, nz));
                    }
                } else {
                    // Infer neighbors for hex (6 directions)
                    let candidate_neighbors = [
                        (q + 1, r, z),
                        (q - 1, r, z),
                        (q, r + 1, z),
                        (q, r - 1, z),
                        (q + 1, r - 1, z),
                        (q - 1, r + 1, z),
                    ];
                    for &(nq, nr, nz) in &candidate_neighbors {
                        if map.contains(&CellKey::Hex {
                            q: nq,
                            r: nr,
                            z: nz,
                        }) {
                            map.add_neighbor((q, r, z), (nq, nr, nz));
                        }
                    }
                }

                if let Some(meta) = cell.get("metadata") {
                    let key = CellKey::Hex { q, r, z };
                    map.set_cell_metadata(&key, meta.clone());
                }
            }
            Some(Map::new(Box::new(map)))
        }

        "province" => {
            // Neighbors must be explicit for province maps
            let mut map = ProvinceMap::new();
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
                    let key = CellKey::Province { id: id.clone() };
                    map.set_cell_metadata(&key, meta.clone());
                }
            }
            Some(Map::new(Box::new(map)))
        }

        _ => None,
    }
}
