//! Crafting system: tool-gated, material/quality-aware, skill-gated production
//! of Item entities.
//!
//! Composes with (never duplicates) the Manufacturing stack (`EconomicSystem`
//! + `ProductionJob` + `Stockpile`): crafting consumes stockpile counts and
//! inventory tools through its own `CraftOrder` component and never writes
//! `Stockpile.resources` outputs. Stockpile-only recipes (no `output_item`)
//! stay on the `EconomicSystem` path and are unknown to the craft path.
//!
//! Follows the [`EcosystemSystem`](super::ecosystem) collect-then-apply
//! pattern: `CraftOrder` IDs are collected sorted ascending, progress is
//! decided on pure reads, and a single apply phase writes `CraftOrder`,
//! spawns output entities, grants XP, and emits events. All stochastic draws
//! come from `deterministic_rng(crafter_id, turn)` with the
//! [`EcosystemSystem`](super::ecosystem) seed layout, so ticks are
//! deterministic. `rand::rng()` is never called on this path.

use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::systems::economic::recipe::Recipe;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde_json::{Value as JsonValue, json};

/// Deterministic crafting system ticking `CraftOrder` components.
pub struct CraftingSystem;

impl System for CraftingSystem {
    fn name(&self) -> &'static str {
        "CraftingSystem"
    }

    fn run(&mut self, world: &mut World) {
        let mut entity_ids = world.get_entities_with_component("CraftOrder");
        entity_ids.sort_unstable();

        let turn = world.turn;
        let mut pending: Vec<(u32, String, Recipe, i64)> = Vec::new();
        for eid in entity_ids {
            let Some(order) = world.get_component(eid, "CraftOrder").cloned() else {
                continue;
            };
            if order.get("state").and_then(|v| v.as_str()) != Some("in_progress") {
                continue;
            }
            let Some(recipe_name) = order
                .get("recipe")
                .and_then(|v| v.as_str())
                .map(str::to_string)
            else {
                continue;
            };
            let Some(recipe) = world.craft_recipes.get(&recipe_name).cloned() else {
                continue;
            };
            if recipe.output_item.is_none() {
                continue;
            }
            let progress = order.get("progress").map(json_int).unwrap_or(0) + 1;
            pending.push((eid, recipe_name, recipe, progress));
        }

        for (eid, recipe_name, recipe, progress) in pending {
            if progress >= recipe.duration {
                complete_craft(world, eid, &recipe_name, &recipe, progress, turn);
            } else {
                let mut order = world
                    .get_component(eid, "CraftOrder")
                    .cloned()
                    .unwrap_or_else(fresh_order);
                order["progress"] = json!(progress);
                let _ = world.set_component(eid, "CraftOrder", order);
            }
        }
    }
}

/// Deterministic RNG seeded from (entity_id, turn); never touches any shared
/// stream. Seed layout matches `ecosystem.rs`: entity bytes into `seed[0..4]`,
/// turn bytes into `seed[4..8]`, `SmallRng::from_seed`.
pub(crate) fn deterministic_rng(entity_id: u32, turn: u32) -> SmallRng {
    let mut seed = [0u8; 32];
    seed[0..4].copy_from_slice(&entity_id.to_le_bytes());
    seed[4..8].copy_from_slice(&turn.to_le_bytes());
    SmallRng::from_seed(seed)
}

/// JSON number tolerant to int/float storage: ints fail `as_f64`, so fall
/// back explicitly instead of silently reading zero.
fn json_num(value: &JsonValue) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| value.as_i64().unwrap_or(0) as f64)
}

/// Integer variant of [`json_num`] for stockpile counts and progress.
fn json_int(value: &JsonValue) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| value.as_f64().unwrap_or(0.0) as i64)
}

/// Blank `CraftOrder` value used only when the stored order vanished between
/// the read and apply phases; never surfaces in normal operation.
fn fresh_order() -> JsonValue {
    json!({"recipe": "", "progress": 0, "state": "in_progress", "output_entity": null})
}

/// Craft-path recipe lookup: unknown names and stockpile-only recipes (no
/// `output_item`) are both unknown to this path.
fn craft_recipe(world: &World, name: &str) -> Option<Recipe> {
    world
        .craft_recipes
        .get(name)
        .filter(|recipe| recipe.output_item.is_some())
        .cloned()
}

/// Current level of `skill` on `crafter`; missing components read as 0.
fn skill_level(world: &World, crafter: u32, skill: &str) -> f64 {
    world
        .get_component(crafter, "SkillLevels")
        .and_then(|levels| levels.get("skills"))
        .and_then(|skills| skills.get(skill))
        .map(json_num)
        .unwrap_or(0.0)
}

/// True when `item` is present on the crafter in any recognized holding:
/// own `Item` component id, `Inventory` slot (string id or object with `id`),
/// `Stockpile` metadata key, or a positive `Stockpile.resources` count.
pub(crate) fn has_tool(world: &World, crafter: u32, item: &str) -> bool {
    if world
        .get_component(crafter, "Item")
        .and_then(|component| component.get("id"))
        .and_then(|id| id.as_str())
        == Some(item)
    {
        return true;
    }
    if let Some(slots) = world
        .get_component(crafter, "Inventory")
        .and_then(|inventory| inventory.get("slots"))
        .and_then(|slots| slots.as_array())
        && slots.iter().any(|slot| {
            slot.as_str() == Some(item) || slot.get("id").and_then(|id| id.as_str()) == Some(item)
        })
    {
        return true;
    }
    if let Some(stockpile) = world.get_component(crafter, "Stockpile") {
        if stockpile
            .as_object()
            .is_some_and(|object| object.keys().any(|key| key == item && key != "resources"))
        {
            return true;
        }
        if stockpile
            .get("resources")
            .and_then(|resources| resources.as_object())
            .and_then(|resources| resources.get(item))
            .is_some_and(|count| json_int(count) > 0)
        {
            return true;
        }
    }
    false
}

/// Shared gate for `can_craft`/`start_craft`: fixed validation order
/// inputs, materials, tools, skill. Returns the recipe on success.
pub(crate) fn validate_craft(
    world: &World,
    crafter: u32,
    recipe_name: &str,
) -> Result<Recipe, String> {
    let recipe = craft_recipe(world, recipe_name).ok_or_else(|| "unknown_recipe".to_string())?;
    if world
        .get_component(crafter, "CraftOrder")
        .and_then(|order| order.get("state"))
        .and_then(|state| state.as_str())
        == Some("in_progress")
    {
        return Err("already_crafting".to_string());
    }
    let resources = world
        .get_component(crafter, "Stockpile")
        .and_then(|stockpile| stockpile.get("resources"))
        .and_then(|resources| resources.as_object());
    for input in &recipe.inputs {
        let current = resources
            .and_then(|map| map.get(&input.kind))
            .map(json_int)
            .unwrap_or(0);
        if current < input.amount {
            return Err(format!("missing_input:{}", input.kind));
        }
    }
    for material in &recipe.materials {
        let current = resources
            .and_then(|map| map.get(&material.material))
            .map(json_int)
            .unwrap_or(0);
        if current < material.amount {
            return Err(format!("missing_material:{}", material.material));
        }
    }
    for tool in &recipe.tools {
        if !has_tool(world, crafter, &tool.item) {
            return Err(format!("missing_tool:{}", tool.item));
        }
    }
    if let Some(required) = &recipe.required_skill
        && skill_level(world, crafter, &required.skill) < required.level as f64
    {
        return Err("insufficient_skill".to_string());
    }
    Ok(recipe)
}

/// Remove a single instance of `item` from the crafter, preferring inventory
/// slots, then the own `Item` component, then stockpile holdings.
fn remove_one_tool(world: &mut World, crafter: u32, item: &str) {
    if let Some(inventory) = world.get_component(crafter, "Inventory").cloned()
        && let Some(slots) = inventory.get("slots").and_then(|slots| slots.as_array())
        && let Some(index) = slots.iter().position(|slot| {
            slot.as_str() == Some(item) || slot.get("id").and_then(|id| id.as_str()) == Some(item)
        })
    {
        let mut updated = inventory.clone();
        if let Some(array) = updated
            .get_mut("slots")
            .and_then(|slots| slots.as_array_mut())
        {
            array.remove(index);
        }
        if world.set_component(crafter, "Inventory", updated).is_ok() {
            return;
        }
    }
    if world
        .get_component(crafter, "Item")
        .and_then(|component| component.get("id"))
        .and_then(|id| id.as_str())
        == Some(item)
        && world.remove_component(crafter, "Item").is_ok()
    {
        return;
    }
    if let Some(stockpile) = world.get_component(crafter, "Stockpile").cloned() {
        let mut updated = stockpile.clone();
        let mut removed = false;
        if let Some(object) = updated.as_object_mut() {
            if object.contains_key(item) && item != "resources" {
                object.remove(item);
                removed = true;
            }
            if !removed
                && let Some(count) = object
                    .get_mut("resources")
                    .and_then(|resources| resources.as_object_mut())
                    .and_then(|resources| resources.get_mut(item))
                && json_int(count) > 0
            {
                *count = json!(json_int(count) - 1);
                removed = true;
            }
        }
        if removed {
            let _ = world.set_component(crafter, "Stockpile", updated);
        }
    }
}

/// Deduct inputs/materials from `Stockpile.resources` and remove consumed
/// tools exactly once. Only touches an existing stockpile, so recipes without
/// requirements never create one.
pub(crate) fn consume_craft_inputs(world: &mut World, crafter: u32, recipe: &Recipe) {
    if world.has_component(crafter, "Stockpile")
        && let Some(stockpile) = world.get_component(crafter, "Stockpile").cloned()
    {
        let mut updated = stockpile.clone();
        let mut touched = false;
        if let Some(map) = updated
            .get_mut("resources")
            .and_then(|resources| resources.as_object_mut())
        {
            for input in &recipe.inputs {
                let current = map.get(&input.kind).map(json_int).unwrap_or(0);
                map.insert(input.kind.clone(), json!(current - input.amount));
                touched = true;
            }
            for material in &recipe.materials {
                let current = map.get(&material.material).map(json_int).unwrap_or(0);
                map.insert(material.material.clone(), json!(current - material.amount));
                touched = true;
            }
        }
        if touched {
            let _ = world.set_component(crafter, "Stockpile", updated);
        }
    }
    for tool in recipe.tools.iter().filter(|tool| tool.consumed) {
        remove_one_tool(world, crafter, &tool.item);
    }
}

/// Normative output quality: crafter `Material.quality` (else 1.0) plus
/// 0.1 per crafting level plus the first stream draw as jitter in
/// [-0.5, +0.5), clamped to [0, 10]. JSON numbers are always finite, so the
/// clamp alone rules out NaN.
pub(crate) fn craft_quality(world: &World, crafter: u32, rng: &mut SmallRng) -> f64 {
    let input_quality = world
        .get_component(crafter, "Material")
        .and_then(|material| material.get("quality"))
        .map(json_num)
        .unwrap_or(1.0);
    let skill = world
        .get_component(crafter, "SkillLevels")
        .and_then(|levels| levels.get("skills"))
        .and_then(|skills| skills.get("crafting"))
        .map(json_num)
        .unwrap_or(0.0);
    let jitter: f64 = rng.random::<f64>() - 0.5;
    (input_quality + 0.1 * skill + jitter).clamp(0.0, 10.0)
}

/// Deterministic XP for one completion: recipe `xp` override, else the skill
/// registry `base_xp` for the XP skill, plus the second stream draw as
/// jitter in [-0.5, +0.5), floored, minimum 1. Quality always consumes the
/// first draw, so the two values share one stream without coupling.
pub(crate) fn craft_xp_amount(recipe: &Recipe, xp_skill: &str, rng: &mut SmallRng) -> i64 {
    let base = recipe
        .xp
        .map(|xp| xp as f64)
        .unwrap_or_else(|| crate::systems::job::system::process::base_xp_for_skill(xp_skill));
    let jitter: f64 = rng.random::<f64>() - 0.5;
    (base + jitter).floor().max(1.0) as i64
}

/// XP skill credited on completion: the skill gate when present, else
/// "crafting".
pub(crate) fn craft_xp_skill(recipe: &Recipe) -> &str {
    recipe
        .required_skill
        .as_ref()
        .map(|required| required.skill.as_str())
        .unwrap_or("crafting")
}

/// Deterministic XP grant mirroring the job skill-grant accounting
/// (`total_xp`, `skill_xp`, `skills`) without level-up rolls: crafting has no
/// `leveled_up` event and must stay exactly recomputable. Missing entries are
/// created with skill value 0, matching the missing-means-0 gate.
pub(crate) fn grant_craft_xp(world: &mut World, crafter: u32, skill: &str, amount: i64) {
    let mut levels = world
        .get_component(crafter, "SkillLevels")
        .cloned()
        .unwrap_or_else(
            || json!({"skills": {}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {}}),
        );
    let total = levels.get("total_xp").map(json_num).unwrap_or(0.0) + amount as f64;
    levels["total_xp"] = json!(total);
    let mut skill_xp = levels
        .get("skill_xp")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let current = skill_xp.get(skill).map(json_num).unwrap_or(0.0);
    skill_xp.insert(skill.to_string(), json!(current + amount as f64));
    levels["skill_xp"] = JsonValue::Object(skill_xp);
    let mut skills = levels
        .get("skills")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    if !skills.contains_key(skill) {
        skills.insert(skill.to_string(), json!(0.0));
    }
    levels["skills"] = JsonValue::Object(skills);
    let _ = world.set_component(crafter, "SkillLevels", levels);
}

/// Material key carried by the output: first `materials[].material`, falling
/// back to the output item id when the recipe lists no materials.
pub(crate) fn craft_material_key(recipe: &Recipe) -> String {
    recipe
        .materials
        .first()
        .map(|material| material.material.clone())
        .unwrap_or_else(|| {
            recipe
                .output_item
                .as_ref()
                .map(|output| output.id.clone())
                .unwrap_or_default()
        })
}

/// Spawn the output entity carrying `Item` + `Material`. When the crafter has
/// an `Inventory` with slot capacity, the output item id (string slot, the
/// established item-id convention) is appended via the normal component
/// write; otherwise the entity simply stays a world entity. The spawned
/// entity always persists, so `output_entity` stays addressable.
pub(crate) fn spawn_craft_output(
    world: &mut World,
    crafter: u32,
    recipe: &Recipe,
    material_key: &str,
    quality: f64,
) -> u32 {
    let output = recipe
        .output_item
        .as_ref()
        .expect("craft output requires output_item");
    let entity = world.spawn_entity();
    let _ = world.set_component(
        entity,
        "Item",
        json!({"id": output.id, "name": output.name, "slot": output.slot, "material": material_key}),
    );
    let _ = world.set_component(
        entity,
        "Material",
        json!({"material": material_key, "quality": quality}),
    );
    if let Some(inventory) = world.get_component(crafter, "Inventory").cloned() {
        let len = inventory
            .get("slots")
            .and_then(|slots| slots.as_array())
            .map(|slots| slots.len())
            .unwrap_or(usize::MAX);
        // Absent or null `max_slots` means unbounded; only a real number caps.
        let capacity = match inventory.get("max_slots") {
            None | Some(JsonValue::Null) => usize::MAX,
            Some(value) => json_int(value).max(0) as usize,
        };
        if len < capacity {
            let mut updated = inventory.clone();
            if let Some(slots) = updated
                .get_mut("slots")
                .and_then(|slots| slots.as_array_mut())
            {
                slots.push(json!(output.id));
                let _ = world.set_component(crafter, "Inventory", updated);
            }
        }
    }
    entity
}

/// Apply-phase completion: quality + XP from one stream, output spawn,
/// XP grant, terminal order state, and the `craft_completed` event.
pub(crate) fn complete_craft(
    world: &mut World,
    crafter: u32,
    recipe_name: &str,
    recipe: &Recipe,
    progress: i64,
    turn: u32,
) {
    let mut rng = deterministic_rng(crafter, turn);
    let quality = craft_quality(world, crafter, &mut rng);
    let xp_skill = craft_xp_skill(recipe).to_string();
    let xp = craft_xp_amount(recipe, &xp_skill, &mut rng);
    let material_key = craft_material_key(recipe);
    let output_entity = spawn_craft_output(world, crafter, recipe, &material_key, quality);
    grant_craft_xp(world, crafter, &xp_skill, xp);
    let mut order = world
        .get_component(crafter, "CraftOrder")
        .cloned()
        .unwrap_or_else(fresh_order);
    order["progress"] = json!(progress);
    order["state"] = json!("complete");
    order["output_entity"] = json!(output_entity);
    let _ = world.set_component(crafter, "CraftOrder", order);
    let output = recipe
        .output_item
        .as_ref()
        .expect("craft output requires output_item");
    let _ = world.send_event(
        "craft_completed",
        json!({
            "entity": crafter,
            "recipe": recipe_name,
            "output_entity": output_entity,
            "output_item": {"id": output.id, "name": output.name, "slot": output.slot},
            "material": material_key,
            "quality": quality,
            "xp_gained": xp,
        }),
    );
}
