use super::World;
use serde_json::{Value as JsonValue, json};

/// Designation shape for [`World::designate_zone`].
///
/// Rect designations are Square-topology only (inclusive `x0..=x1`,
/// `y0..=y1` on level `z`); Hex maps use explicit cell lists.
#[derive(Debug, Clone)]
pub enum ZoneShape {
    /// Inclusive rectangle on Square topology. Requires `x0 <= x1`, `y0 <= y1`.
    Rect {
        x0: i64,
        y0: i64,
        z: i64,
        x1: i64,
        y1: i64,
    },
    /// Explicit Square/Hex cell JSON values (or nested `Region` references).
    Cells(Vec<JsonValue>),
}

/// True when the value is a usable zone cell: a Square/Hex/Province cell
/// (bare or `pos`-wrapped) or a nested `Region` reference.
fn is_valid_zone_cell(cell: &JsonValue) -> bool {
    if crate::map::CellKey::from_position(cell).is_some() {
        return true;
    }
    cell.get("Region")
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .is_some()
}

impl World {
    /// Colony gate shared by all mutating zone ops.
    fn zone_gate(&self, op: &str) -> Result<(), String> {
        if self.get_mode() != "colony" {
            return Err(format!(
                "{op}: world is in '{}' mode; zone management requires 'colony' mode",
                self.get_mode()
            ));
        }
        Ok(())
    }

    /// Entity ids carrying a `Zone` record with the given id.
    fn zone_record_entities(&self, zone_id: &str) -> Vec<u32> {
        self.get_entities_with_component("Zone")
            .into_iter()
            .filter(|&eid| {
                self.get_component(eid, "Zone")
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    == Some(zone_id)
            })
            .collect()
    }

    /// Designates a zone, returning its unique id.
    ///
    /// Rect designations persist as one `ZoneRect` record (constant storage
    /// regardless of area); explicit cells persist as `RegionAssignment`
    /// entities keyed by the zone id, so the zone is addressable through the
    /// existing region id query path. Labels are non-unique; only ids are.
    /// Errors on empty `kind`, malformed rects (naming the offending field),
    /// invalid cells, and outside `colony` mode.
    pub fn designate_zone(
        &mut self,
        kind: &str,
        label: Option<&str>,
        shape: ZoneShape,
    ) -> Result<String, String> {
        self.zone_gate("designate_zone")?;
        if kind.is_empty() {
            return Err("designate_zone: 'kind' must be a non-empty string".to_string());
        }
        if let ZoneShape::Rect { x0, x1, .. } = &shape
            && x0 > x1
        {
            return Err(format!(
                "designate_zone: rect 'x0' ({x0}) must be <= 'x1' ({x1})"
            ));
        }
        if let ZoneShape::Rect { y0, y1, .. } = &shape
            && y0 > y1
        {
            return Err(format!(
                "designate_zone: rect 'y0' ({y0}) must be <= 'y1' ({y1})"
            ));
        }
        if let ZoneShape::Cells(cells) = &shape {
            for (i, cell) in cells.iter().enumerate() {
                if !is_valid_zone_cell(cell) {
                    return Err(format!(
                        "designate_zone: cells[{i}] is not a valid Square/Hex cell or Region reference"
                    ));
                }
            }
        }

        let zone_id = format!("zone-{}", self.next_zone_id);
        self.next_zone_id += 1;

        let record = self.spawn_entity();
        self.set_component(
            record,
            "Zone",
            json!({"id": zone_id, "kind": kind, "label": label}),
        )?;

        match shape {
            ZoneShape::Rect { x0, y0, z, x1, y1 } => {
                let rect = self.spawn_entity();
                self.set_component(
                    rect,
                    "ZoneRect",
                    json!({"zone_id": zone_id, "x0": x0, "y0": y0, "z": z, "x1": x1, "y1": y1}),
                )?;
            }
            ZoneShape::Cells(cells) => {
                for cell in cells {
                    let assignment = self.spawn_entity();
                    self.set_component(
                        assignment,
                        "RegionAssignment",
                        json!({"cell": cell, "region_id": zone_id}),
                    )?;
                }
            }
        }
        Ok(zone_id)
    }

    /// Removes a zone and all of its cell assignments.
    ///
    /// Returns `Ok(false)` for unknown ids without touching other zones.
    /// Array-form `RegionAssignment.region_id` values are stripped of the
    /// zone id (entity despawned only when nothing remains); string-form
    /// matches are despawned.
    pub fn remove_zone(&mut self, zone_id: &str) -> Result<bool, String> {
        self.zone_gate("remove_zone")?;
        if self.zone_record_entities(zone_id).is_empty() {
            return Ok(false);
        }
        for eid in self.zone_record_entities(zone_id) {
            self.despawn_entity(eid);
        }
        let rects: Vec<u32> = self
            .get_entities_with_component("ZoneRect")
            .into_iter()
            .filter(|&eid| {
                self.get_component(eid, "ZoneRect")
                    .and_then(|v| v.get("zone_id"))
                    .and_then(|v| v.as_str())
                    == Some(zone_id)
            })
            .collect();
        for eid in rects {
            self.despawn_entity(eid);
        }
        let assignments: Vec<u32> = self
            .get_entities_with_component("RegionAssignment")
            .into_iter()
            .filter(|&eid| {
                let Some(val) = self.get_component(eid, "RegionAssignment") else {
                    return false;
                };
                match val.get("region_id") {
                    Some(JsonValue::String(s)) => s == zone_id,
                    Some(JsonValue::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(zone_id)),
                    _ => false,
                }
            })
            .collect();
        for eid in assignments {
            let strip_to_empty = self
                .get_component(eid, "RegionAssignment")
                .and_then(|v| v.get("region_id"))
                .map(|rid| match rid {
                    JsonValue::Array(arr) => {
                        arr.iter().filter(|v| v.as_str() != Some(zone_id)).count() == 0
                    }
                    _ => true,
                })
                .unwrap_or(true);
            if strip_to_empty {
                self.despawn_entity(eid);
            } else if let Some(val) = self
                .components
                .get_mut("RegionAssignment")
                .and_then(|m| m.get_mut(&eid))
                && let Some(arr) = val.get_mut("region_id").and_then(|v| v.as_array_mut())
            {
                arr.retain(|v| v.as_str() != Some(zone_id));
            }
        }
        Ok(true)
    }

    /// Renames a zone by replacing its label.
    ///
    /// Labels are non-unique; only ids are. Returns `Ok(false)` for unknown
    /// ids without creating entities. Gated to `colony` mode.
    pub fn rename_zone(&mut self, zone_id: &str, label: &str) -> Result<bool, String> {
        self.zone_gate("rename_zone")?;
        let Some(eid) = self.zone_record_entities(zone_id).into_iter().next() else {
            return Ok(false);
        };
        let mut record = self
            .get_component(eid, "Zone")
            .cloned()
            .unwrap_or(json!(null));
        record["label"] = json!(label);
        self.set_component(eid, "Zone", record)?;
        Ok(true)
    }

    /// Changes a zone's kind, rejecting empty kinds naming the field.
    ///
    /// Returns `Ok(false)` for unknown ids without creating entities. Gated
    /// to `colony` mode.
    pub fn set_zone_kind(&mut self, zone_id: &str, kind: &str) -> Result<bool, String> {
        self.zone_gate("set_zone_kind")?;
        if kind.is_empty() {
            return Err("set_zone_kind: 'kind' must be a non-empty string".to_string());
        }
        let Some(eid) = self.zone_record_entities(zone_id).into_iter().next() else {
            return Ok(false);
        };
        let mut record = self
            .get_component(eid, "Zone")
            .cloned()
            .unwrap_or(json!(null));
        record["kind"] = json!(kind);
        self.set_component(eid, "Zone", record)?;
        Ok(true)
    }

    /// Assigns explicit cells to a zone, ignoring cells that are already
    /// members (idempotent).
    ///
    /// Returns `Ok(false)` for unknown ids without creating entities. Invalid
    /// cells are rejected, naming the offending index. Gated to `colony`
    /// mode.
    pub fn assign_cells_to_zone(
        &mut self,
        zone_id: &str,
        cells: Vec<JsonValue>,
    ) -> Result<bool, String> {
        self.zone_gate("assign_cells_to_zone")?;
        if self.zone_record_entities(zone_id).is_empty() {
            return Ok(false);
        }
        for (i, cell) in cells.iter().enumerate() {
            if !is_valid_zone_cell(cell) {
                return Err(format!(
                    "assign_cells_to_zone: cells[{i}] is not a valid Square/Hex cell or Region reference"
                ));
            }
        }
        let member: std::collections::HashSet<String> = self
            .zone_member_cells(zone_id)
            .iter()
            .map(|c| c.to_string())
            .collect();
        for cell in cells {
            if member.contains(&cell.to_string()) {
                continue;
            }
            let assignment = self.spawn_entity();
            self.set_component(
                assignment,
                "RegionAssignment",
                json!({"cell": cell, "region_id": zone_id}),
            )?;
        }
        Ok(true)
    }

    /// Removes explicit cells from a zone, ignoring cells that are not
    /// members (idempotent).
    ///
    /// Rect interiors are unaffected. Array-form `RegionAssignment.region_id`
    /// values are stripped of the zone id (entity despawned only when nothing
    /// remains); string-form matches are despawned. Returns `Ok(false)` for
    /// unknown ids. Gated to `colony` mode.
    pub fn unassign_cells_from_zone(
        &mut self,
        zone_id: &str,
        cells: Vec<JsonValue>,
    ) -> Result<bool, String> {
        self.zone_gate("unassign_cells_from_zone")?;
        if self.zone_record_entities(zone_id).is_empty() {
            return Ok(false);
        }
        let targets: std::collections::HashSet<String> =
            cells.iter().map(|c| c.to_string()).collect();
        if targets.is_empty() {
            return Ok(true);
        }
        let assignments: Vec<u32> = self
            .get_entities_with_component("RegionAssignment")
            .into_iter()
            .filter(|&eid| {
                let Some(val) = self.get_component(eid, "RegionAssignment") else {
                    return false;
                };
                let zone_matches = match val.get("region_id") {
                    Some(JsonValue::String(s)) => s == zone_id,
                    Some(JsonValue::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(zone_id)),
                    _ => false,
                };
                zone_matches
                    && val
                        .get("cell")
                        .is_some_and(|cell| targets.contains(&cell.to_string()))
            })
            .collect();
        for eid in assignments {
            let strip_to_empty = self
                .get_component(eid, "RegionAssignment")
                .and_then(|v| v.get("region_id"))
                .map(|rid| match rid {
                    JsonValue::Array(arr) => {
                        arr.iter().filter(|v| v.as_str() != Some(zone_id)).count() == 0
                    }
                    _ => true,
                })
                .unwrap_or(true);
            if strip_to_empty {
                self.despawn_entity(eid);
            } else if let Some(val) = self
                .components
                .get_mut("RegionAssignment")
                .and_then(|m| m.get_mut(&eid))
                && let Some(arr) = val.get_mut("region_id").and_then(|v| v.as_array_mut())
            {
                arr.retain(|v| v.as_str() != Some(zone_id));
            }
        }
        Ok(true)
    }

    /// Validates zone state, dropping references to removed zones.
    ///
    /// Orphan `ZoneRect` records (whose `zone_id` has no live `Zone`
    /// record) are despawned. `RegionAssignment` references to zone ids
    /// (`zone-` prefix) with no live `Zone` record are dropped:
    /// string-form matches are despawned, array-form values are stripped
    /// of the stale id (entity despawned only when nothing remains).
    /// Assignments carrying only plain region ids are never touched, an
    /// empty zone set is a safe no-op, and the pass never errors. Called
    /// each tick by `ZoneSystem`; ungated by world mode.
    pub fn validate_zones(&mut self) {
        let live: std::collections::HashSet<String> = self
            .get_entities_with_component("Zone")
            .iter()
            .filter_map(|&eid| {
                self.get_component(eid, "Zone")?
                    .get("id")?
                    .as_str()
                    .map(str::to_string)
            })
            .collect();
        let orphans: Vec<u32> = self
            .get_entities_with_component("ZoneRect")
            .into_iter()
            .filter(|&eid| {
                self.get_component(eid, "ZoneRect")
                    .and_then(|v| v.get("zone_id"))
                    .and_then(|v| v.as_str())
                    .is_none_or(|id| !live.contains(id))
            })
            .collect();
        for eid in orphans {
            self.despawn_entity(eid);
        }
        let assignments: Vec<u32> = self
            .get_entities_with_component("RegionAssignment")
            .into_iter()
            .collect();
        for eid in assignments {
            let Some(val) = self.get_component(eid, "RegionAssignment").cloned() else {
                continue;
            };
            match val.get("region_id") {
                Some(JsonValue::String(id)) if id.starts_with("zone-") && !live.contains(id) => {
                    self.despawn_entity(eid);
                }
                Some(JsonValue::Array(arr)) => {
                    let kept: Vec<JsonValue> = arr
                        .iter()
                        .filter(|v| match v.as_str() {
                            Some(id) if id.starts_with("zone-") => live.contains(id),
                            _ => true,
                        })
                        .cloned()
                        .collect();
                    if kept.len() == arr.len() {
                        continue;
                    }
                    if kept.is_empty() {
                        self.despawn_entity(eid);
                    } else {
                        let mut next = val.clone();
                        next["region_id"] = json!(kept);
                        let _ = self.set_component(eid, "RegionAssignment", next);
                    }
                }
                _ => {}
            }
        }
    }

    /// Explicit cells directly assigned to a zone (raw stored values).
    fn zone_member_cells(&self, zone_id: &str) -> Vec<JsonValue> {
        let mut cells = Vec::new();
        for eid in self.get_entities_with_component("RegionAssignment") {
            let Some(val) = self.get_component(eid, "RegionAssignment") else {
                continue;
            };
            let matches = match val.get("region_id") {
                Some(JsonValue::String(s)) => s == zone_id,
                Some(JsonValue::Array(arr)) => arr.iter().any(|v| v.as_str() == Some(zone_id)),
                _ => false,
            };
            if matches && let Some(cell) = val.get("cell").cloned() {
                cells.push(cell);
            }
        }
        cells
    }

    /// Lists all zones as `{id, label, kind, cell_count}` sorted by id.    ///
    /// `cell_count` is the expanded cell count from
    /// [`cells_in_region`](Self::cells_in_region) (rect expansion plus
    /// explicit cells with nested `Region` resolution). Read path: ungated.
    pub fn list_zones(&self) -> Vec<JsonValue> {
        let mut zones: Vec<(String, JsonValue)> = self
            .get_entities_with_component("Zone")
            .into_iter()
            .filter_map(|eid| {
                let val = self.get_component(eid, "Zone")?.clone();
                let id = val.get("id")?.as_str()?.to_string();
                Some((id, val))
            })
            .collect();
        zones.sort_by(|a, b| a.0.cmp(&b.0));
        zones
            .into_iter()
            .map(|(id, val)| {
                json!({
                    "id": id,
                    "label": val.get("label").cloned().unwrap_or(JsonValue::Null),
                    "kind": val.get("kind").cloned().unwrap_or(JsonValue::Null),
                    "cell_count": self.cells_in_region(&id).len(),
                })
            })
            .collect()
    }

    /// Returns `{id, label, kind, rects, cells}` for a zone, or `None`.
    ///
    /// `rects` are the stored `ZoneRect` records; `cells` are the directly
    /// assigned explicit cells (raw stored values, no expansion). Read path:
    /// ungated.
    pub fn get_zone(&self, zone_id: &str) -> Option<JsonValue> {
        let record = self
            .zone_record_entities(zone_id)
            .into_iter()
            .next()
            .and_then(|eid| self.get_component(eid, "Zone").cloned())?;
        let mut rects: Vec<JsonValue> = self
            .get_entities_with_component("ZoneRect")
            .into_iter()
            .filter_map(|eid| {
                let val = self.get_component(eid, "ZoneRect")?;
                if val.get("zone_id")?.as_str()? != zone_id {
                    return None;
                }
                Some(json!({
                    "x0": val.get("x0"),
                    "y0": val.get("y0"),
                    "z": val.get("z"),
                    "x1": val.get("x1"),
                    "y1": val.get("y1"),
                }))
            })
            .collect();
        rects.sort_by_key(|a| a.to_string());
        let cells = self.zone_member_cells(zone_id);
        Some(json!({
            "id": record.get("id"),
            "label": record.get("label").cloned().unwrap_or(JsonValue::Null),
            "kind": record.get("kind"),
            "rects": rects,
            "cells": cells,
        }))
    }

    /// Expands the `ZoneRect` records of a zone id into Square cell values.
    ///
    /// Called from [`cells_in_region`](Self::cells_in_region) via
    /// `collect_region_cells`; cells already present in `out` are skipped so
    /// rect interiors never duplicate explicit assignments.
    pub(crate) fn expand_zone_rects(
        &self,
        zone_id: &str,
        out: &mut Vec<JsonValue>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        for eid in self.get_entities_with_component("ZoneRect") {
            let Some(val) = self.get_component(eid, "ZoneRect") else {
                continue;
            };
            if val.get("zone_id").and_then(|v| v.as_str()) != Some(zone_id) {
                continue;
            }
            let (Some(x0), Some(y0), Some(z), Some(x1), Some(y1)) = (
                val.get("x0").and_then(|v| v.as_i64()),
                val.get("y0").and_then(|v| v.as_i64()),
                val.get("z").and_then(|v| v.as_i64()),
                val.get("x1").and_then(|v| v.as_i64()),
                val.get("y1").and_then(|v| v.as_i64()),
            ) else {
                continue;
            };
            for x in x0..=x1 {
                for y in y0..=y1 {
                    let cell = json!({"Square": {"x": x, "y": y, "z": z}});
                    if seen.insert(cell.to_string()) {
                        out.push(cell);
                    }
                }
            }
        }
    }
}
