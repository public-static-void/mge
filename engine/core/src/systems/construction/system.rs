use crate::World;
use crate::ecs::system::System;
use crate::map::CellKey;
use serde_json::{Value as JsonValue, json};

/// Serialize a [`CellKey`] as a `Position`-shaped component value.
///
/// Produces `{ "pos": { "Square": ... } }` (or `Hex` / `Province`), the same
/// shape used by `Position`, `ConstructionSite.target_position`, and
/// `Job.target_position`, so [`CellKey::from_position`] round-trips it.
pub fn cell_position_json(cell: &CellKey) -> JsonValue {
    match cell {
        CellKey::Square { x, y, z } => json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
        CellKey::Hex { q, r, z } => json!({ "pos": { "Hex": { "q": q, "r": r, "z": z } } }),
        CellKey::Province { id } => json!({ "pos": { "Province": { "id": id } } }),
    }
}

/// Returns true when the cell variant matches the active map topology.
fn topology_matches(cell: &CellKey, topology_type: &str) -> bool {
    match cell {
        CellKey::Square { .. } => topology_type == "square",
        CellKey::Hex { .. } => topology_type == "hex",
        CellKey::Province { .. } => topology_type == "province",
    }
}

/// Validates a blueprint and spawns the construction ghost plus linked job.
///
/// Rejects when the world is not in colony mode, the material list is empty
/// or holds an `amount < 1`, `required_work < 1`, no map is active, the cell
/// topology differs from the active map topology, the cell is absent from the
/// active map, or a `Building` / `ConstructionSite` already occupies the cell
/// (same x,y,z for Square; same q,r,z for Hex; same province id, capacity 1).
/// Same-x,y-different-z cells are distinct and accepted.
///
/// On success spawns exactly one ghost entity (`Position` + `ConstructionSite`
/// in `pending` state) and exactly one linked `Job` with
/// `category="construction"` and `job_type="construct"` whose
/// `resource_requirements` mirror the material list, and returns the site id.
pub fn place_blueprint(
    world: &mut World,
    building_type: &str,
    cell: &CellKey,
    required_materials: &[(String, i64)],
    required_work: i64,
) -> Result<u32, String> {
    if world.get_mode() != "colony" {
        return Err(format!(
            "place_blueprint: world is in '{}' mode; construction requires 'colony' mode",
            world.get_mode()
        ));
    }
    if required_materials.is_empty() {
        return Err("place_blueprint: required_materials must not be empty".to_string());
    }
    for (kind, amount) in required_materials {
        if *amount < 1 {
            return Err(format!(
                "place_blueprint: material '{kind}' has amount {amount}; amounts must be >= 1"
            ));
        }
    }
    if required_work < 1 {
        return Err(format!(
            "place_blueprint: required_work is {required_work}; must be >= 1"
        ));
    }
    let Some(map) = world.get_map() else {
        return Err("place_blueprint: no active map".to_string());
    };
    if !topology_matches(cell, map.topology_type()) {
        return Err(format!(
            "place_blueprint: cell topology does not match active map topology '{}'",
            map.topology_type()
        ));
    }
    if !map.contains(cell) {
        return Err("place_blueprint: target cell is not on the active map".to_string());
    }
    let occupied = world.entities_in_cell(cell).into_iter().any(|eid| {
        world.has_component(eid, "Building") || world.has_component(eid, "ConstructionSite")
    });
    if occupied {
        return Err("place_blueprint: target cell is already occupied".to_string());
    }

    let pos_json = cell_position_json(cell);
    let materials_json: Vec<JsonValue> = required_materials
        .iter()
        .map(|(kind, amount)| json!({ "kind": kind, "amount": amount }))
        .collect();

    let site_id = world.spawn_entity();
    let site_result = world
        .set_component(site_id, "Position", pos_json.clone())
        .and_then(|()| {
            world.set_component(
                site_id,
                "ConstructionSite",
                json!({
                    "building_type": building_type,
                    "target_position": pos_json,
                    "required_materials": materials_json.clone(),
                    "progress": 0,
                    "required_work": required_work,
                    "state": "pending",
                    "reserved_stockpile": null,
                    "assigned_job": null,
                }),
            )
        });
    if let Err(e) = site_result {
        world.despawn_entity(site_id);
        return Err(format!(
            "place_blueprint: failed to spawn ghost entity: {e}"
        ));
    }

    let job_id = world.spawn_entity();
    let job_result = world.set_component(
        job_id,
        "Job",
        json!({
            "id": job_id,
            "job_type": "construct",
            "category": "construction",
            "state": "pending",
            "target": site_id,
            "target_position": cell_position_json(cell),
            "resource_requirements": materials_json,
            "required_progress": required_work,
            "priority": 0,
            "created_at": world.turn,
        }),
    );
    if let Err(e) = job_result {
        world.despawn_entity(job_id);
        world.despawn_entity(site_id);
        return Err(format!(
            "place_blueprint: failed to post construction job: {e}"
        ));
    }

    if let Some(mut site) = world.get_component(site_id, "ConstructionSite").cloned() {
        site["assigned_job"] = json!(job_id);
        if let Err(e) = world.set_component(site_id, "ConstructionSite", site) {
            world.despawn_entity(job_id);
            world.despawn_entity(site_id);
            return Err(format!("place_blueprint: failed to link job to site: {e}"));
        }
    }

    Ok(site_id)
}

/// Returns the observable construction state for a site.
///
/// In colony mode returns `{ state, progress, required_work, building_type }`
/// from the live `ConstructionSite` component; for a completed site (which
/// carries `Building` instead) returns `state: "complete"` with the building
/// type and `progress`/`required_work` derived from `integrity`. Unknown ids
/// and non-colony modes are errors.
pub fn get_construction_state(world: &World, site_id: u32) -> Result<JsonValue, String> {
    if world.get_mode() != "colony" {
        return Err(format!(
            "get_construction_state: world is in '{}' mode; construction requires 'colony' mode",
            world.get_mode()
        ));
    }
    if let Some(site) = world.get_component(site_id, "ConstructionSite") {
        let state = site
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string();
        let progress = site.get("progress").and_then(|v| v.as_i64()).unwrap_or(0);
        let required_work = site
            .get("required_work")
            .and_then(|v| v.as_i64())
            .unwrap_or(1);
        let building_type = site
            .get("building_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        return Ok(json!({
            "state": state,
            "progress": progress,
            "required_work": required_work,
            "building_type": building_type,
        }));
    }
    if let Some(building) = world.get_component(site_id, "Building") {
        let building_type = building
            .get("building_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let integrity = building
            .get("integrity")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        return Ok(json!({
            "state": "complete",
            "progress": integrity,
            "required_work": integrity,
            "building_type": building_type,
        }));
    }
    Err(format!(
        "get_construction_state: unknown construction site {site_id}"
    ))
}

/// Cancels a pre-completion construction site.
///
/// Rejects unknown ids, sites already in a terminal state (`complete` /
/// `cancelled`), and completed buildings (which carry `Building` instead of
/// `ConstructionSite` — use [`demolish_building`] for those). On success the
/// linked job's reservation is cleared (`reserved_resources` emptied,
/// `reserved_stockpile` nulled so the stateless reservation pass cannot
/// re-reserve it), already-delivered materials are refunded to the reserving
/// stockpile, the linked job is flipped to `cancelled` (the next `JobSystem`
/// tick then releases the worker and emits `job_cancelled` via the existing
/// cancellation path), and the ghost entity is despawned. Returns `true`.
pub fn cancel_construction(world: &mut World, site_id: u32) -> Result<bool, String> {
    let site = match world.get_component(site_id, "ConstructionSite").cloned() {
        Some(site) => site,
        None => {
            if world.has_component(site_id, "Building") {
                return Err(format!(
                    "cancel_construction: site {site_id} is already complete; use demolish_building"
                ));
            }
            return Err(format!(
                "cancel_construction: unknown construction site {site_id}"
            ));
        }
    };
    let state = site.get("state").and_then(|v| v.as_str()).unwrap_or("");
    if matches!(state, "complete" | "cancelled") {
        return Err(format!(
            "cancel_construction: site {site_id} is already '{state}'; use demolish_building for completed buildings"
        ));
    }

    if let Some(job_id) = site
        .get("assigned_job")
        .and_then(|v| v.as_u64())
        .map(|id| id as u32)
        && let Some(mut job) = world.get_component(job_id, "Job").cloned()
    {
        refund_delivered(world, &site, &job);
        if let Some(obj) = job.as_object_mut() {
            obj.insert("reserved_resources".to_string(), json!([]));
            obj.insert("reserved_stockpile".to_string(), JsonValue::Null);
            obj.insert("state".to_string(), json!("cancelled"));
        }
        let _ = world.set_component(job_id, "Job", job);
    }

    world.despawn_entity(site_id);
    Ok(true)
}

/// Adds the linked job's `delivered_resources` back to the reserving
/// stockpile (the site's `reserved_stockpile`, falling back to the job's).
/// Integer-only, matching the consume-on-delivery accounting. Missing
/// stockpiles or empty deliveries are no-ops.
fn refund_delivered(world: &mut World, site: &JsonValue, job: &JsonValue) {
    let delivered: Vec<JsonValue> = job
        .get("delivered_resources")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if delivered.is_empty() {
        return;
    }
    let stockpile_id = site
        .get("reserved_stockpile")
        .and_then(|v| v.as_u64())
        .or_else(|| job.get("reserved_stockpile").and_then(|v| v.as_u64()))
        .map(|id| id as u32);
    let Some(stockpile_id) = stockpile_id else {
        return;
    };
    let Some(mut stockpile) = world.get_component(stockpile_id, "Stockpile").cloned() else {
        return;
    };
    let Some(resources) = stockpile
        .get_mut("resources")
        .and_then(|v| v.as_object_mut())
    else {
        return;
    };
    for item in &delivered {
        let kind = item.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        if kind.is_empty() {
            continue;
        }
        let amount = item.get("amount").map(amount_as_i64).unwrap_or(0);
        if amount <= 0 {
            continue;
        }
        let available = resources.get(kind).map(amount_as_i64).unwrap_or(0);
        resources.insert(kind.to_string(), json!(available + amount));
    }
    let _ = world.set_component(stockpile_id, "Stockpile", stockpile);
}

/// Demolishes a completed building with no material refund.
///
/// Only entities carrying `Building` are accepted: in-progress sites (which
/// carry `ConstructionSite`) are rejected so active jobs are never disturbed,
/// and unknown ids are rejected. The linked construction job is already
/// `complete` (its worker was released at completion) and is left in place as
/// history. Returns `true`.
pub fn demolish_building(world: &mut World, building_id: u32) -> Result<bool, String> {
    if world.has_component(building_id, "ConstructionSite") {
        return Err(format!(
            "demolish_building: entity {building_id} is an in-progress construction site; use cancel_construction"
        ));
    }
    if !world.has_component(building_id, "Building") {
        return Err(format!("demolish_building: unknown building {building_id}"));
    }
    world.despawn_entity(building_id);
    Ok(true)
}

/// Reads a JSON amount as a non-negative integer with no float conversion.
fn amount_as_i64(value: &JsonValue) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|u| i64::try_from(u).ok()))
        .unwrap_or(0)
        .max(0)
}

/// True when every requirement kind has at least the needed delivered amount.
fn requirements_met(requirements: &[JsonValue], delivered: &[JsonValue]) -> bool {
    crate::systems::job::states::transitions::are_requirements_met(requirements, delivered)
}

/// Construction progress and completion system.
///
/// Runs in the explicit `SYSTEM_EXECUTION_ORDER` slot immediately after
/// `EconomicSystem`. Each tick, for every non-terminal `ConstructionSite` in
/// ascending entity order:
/// - mirrors the linked job's reservation/delivery phase into site state
///   (`pending` → `reserved` → `delivering`),
/// - on full delivery consumes the required materials from the reserving
///   stockpile exactly once (guarded by the `in_progress` transition; work
///   ticks never touch the stockpile) and flips the site to `in_progress`,
/// - while a worker is at the site cell increments `progress` by 1 per tick,
///   completing at `progress >= required_work` by emitting
///   `construction_completed` and replacing `ConstructionSite` with `Building`
///   on the same entity (position preserved).
#[derive(Default)]
pub struct ConstructionSystem;

impl ConstructionSystem {
    /// Creates a new construction system.
    pub fn new() -> Self {
        Self
    }
}

impl System for ConstructionSystem {
    fn name(&self) -> &'static str {
        "ConstructionSystem"
    }

    fn run(&mut self, world: &mut World) {
        let mut sites = world.get_entities_with_component("ConstructionSite");
        sites.sort_unstable();
        for site_id in sites {
            Self::tick_site(world, site_id);
        }
    }
}

impl ConstructionSystem {
    /// Advances one construction site by a single tick.
    fn tick_site(world: &mut World, site_id: u32) {
        let Some(mut site) = world.get_component(site_id, "ConstructionSite").cloned() else {
            return;
        };
        let state = site.get("state").and_then(|v| v.as_str()).unwrap_or("");
        if matches!(state, "complete" | "cancelled") {
            return;
        }
        let job_id = match site.get("assigned_job").and_then(|v| v.as_u64()) {
            Some(id) => id as u32,
            None => return,
        };
        let Some(job) = world.get_component(job_id, "Job").cloned() else {
            return;
        };
        let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
        if matches!(job_state, "cancelled" | "failed" | "blocked") {
            return;
        }

        let requirements: Vec<JsonValue> = site
            .get("required_materials")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let delivered: Vec<JsonValue> = job
            .get("delivered_resources")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if !requirements_met(&requirements, &delivered) {
            Self::mirror_pre_delivery_state(world, site_id, &mut site, &job);
            return;
        }

        if state != "in_progress" {
            Self::consume_on_delivery(world, &site, &job, &requirements);
            site["state"] = json!("in_progress");
            let _ = world.set_component(site_id, "ConstructionSite", site.clone());
        }

        if !Self::worker_at_site(world, site_id, &job) {
            return;
        }

        let progress = site.get("progress").and_then(|v| v.as_i64()).unwrap_or(0) + 1;
        let required_work = site
            .get("required_work")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1);
        site["progress"] = json!(progress);
        if progress >= required_work {
            Self::complete_site(world, site_id, job_id, &mut site, &requirements);
        } else {
            let _ = world.set_component(site_id, "ConstructionSite", site);
        }
    }

    /// Mirrors the linked job's pre-delivery phase into site state and
    /// reservation fields without touching progress or stockpiles.
    fn mirror_pre_delivery_state(
        world: &mut World,
        site_id: u32,
        site: &mut JsonValue,
        job: &JsonValue,
    ) {
        let reserved_stockpile = job
            .get("reserved_stockpile")
            .cloned()
            .unwrap_or(JsonValue::Null);
        let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
        let next_state = if reserved_stockpile.is_number() {
            if matches!(
                job_state,
                "fetching_resources" | "delivering_resources" | "going_to_site" | "at_site"
            ) {
                "delivering"
            } else {
                "reserved"
            }
        } else {
            "pending"
        };
        let mut changed = false;
        if site.get("state").and_then(|v| v.as_str()) != Some(next_state) {
            site["state"] = json!(next_state);
            changed = true;
        }
        if site.get("reserved_stockpile") != Some(&reserved_stockpile) {
            site["reserved_stockpile"] = reserved_stockpile;
            changed = true;
        }
        if changed {
            let _ = world.set_component(site_id, "ConstructionSite", site.clone());
        }
    }

    /// Decrements the reserving stockpile by the required materials, clamping
    /// at zero so schema `minimum: 0` validation always holds. Called only on
    /// the `delivering` → `in_progress` transition, so each site consumes
    /// exactly once no matter how delivery interleaves with work ticks.
    fn consume_on_delivery(
        world: &mut World,
        site: &JsonValue,
        job: &JsonValue,
        requirements: &[JsonValue],
    ) {
        let stockpile_id = site
            .get("reserved_stockpile")
            .and_then(|v| v.as_u64())
            .or_else(|| job.get("reserved_stockpile").and_then(|v| v.as_u64()))
            .map(|id| id as u32);
        let Some(stockpile_id) = stockpile_id else {
            return;
        };
        let Some(mut stockpile) = world.get_component(stockpile_id, "Stockpile").cloned() else {
            return;
        };
        let Some(resources) = stockpile
            .get_mut("resources")
            .and_then(|v| v.as_object_mut())
        else {
            return;
        };
        for req in requirements {
            let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            if kind.is_empty() {
                continue;
            }
            let amount = req.get("amount").map(amount_as_i64).unwrap_or(0);
            let available = resources.get(kind).map(amount_as_i64).unwrap_or(0);
            resources.insert(kind.to_string(), json!((available - amount).max(0)));
        }
        let _ = world.set_component(stockpile_id, "Stockpile", stockpile);
    }

    /// True when a worker stands on the site cell: the assigned agent when the
    /// linked job still holds one, otherwise any agent (covers the case where
    /// the job completed early and released its worker while it idles on site).
    fn worker_at_site(world: &World, site_id: u32, job: &JsonValue) -> bool {
        let Some(site_pos) = world.get_component(site_id, "Position") else {
            return false;
        };
        let Some(site_cell) = CellKey::from_position(site_pos) else {
            return false;
        };
        if let Some(agent_id) = job
            .get("assigned_to")
            .and_then(|v| v.as_u64())
            .map(|id| id as u32)
        {
            return world
                .get_component(agent_id, "Position")
                .and_then(CellKey::from_position)
                .is_some_and(|cell| cell == site_cell);
        }
        world
            .get_entities_with_component("Agent")
            .into_iter()
            .any(|eid| {
                world
                    .get_component(eid, "Position")
                    .and_then(CellKey::from_position)
                    .is_some_and(|cell| cell == site_cell)
            })
    }

    /// Completes the site: emits `construction_completed`, replaces
    /// `ConstructionSite` with `Building` on the same entity (position
    /// preserved, `materials_used` recorded), and marks the linked job
    /// complete when it has not already reached a terminal state.
    fn complete_site(
        world: &mut World,
        site_id: u32,
        job_id: u32,
        site: &mut JsonValue,
        requirements: &[JsonValue],
    ) {
        let building_type = site
            .get("building_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let required_work = site
            .get("required_work")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1);
        let position = world
            .get_component(site_id, "Position")
            .cloned()
            .unwrap_or(json!(null));

        site["state"] = json!("complete");
        site["progress"] = json!(required_work);
        let _ = world.set_component(site_id, "ConstructionSite", site.clone());
        let _ = world.remove_component(site_id, "ConstructionSite");
        let _ = world.set_component(
            site_id,
            "Building",
            json!({
                "building_type": building_type,
                "materials_used": requirements,
                "integrity": required_work,
                "passable": false,
                "blocks_sight": false,
            }),
        );

        if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
            let job_state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            if !matches!(
                job_state,
                "complete" | "failed" | "cancelled" | "blocked" | "interrupted"
            ) {
                job["state"] = json!("complete");
                job["progress"] = json!(required_work);
                let _ = world.set_component(job_id, "Job", job);
            }
        }

        let _ = world.send_event(
            "construction_completed",
            json!({
                "site_id": site_id,
                "building_id": site_id,
                "building_type": building_type,
                "position": position,
            }),
        );
    }
}
