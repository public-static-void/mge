#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::body_equipment_sync::BodyEquipmentSyncSystem;
use engine_core::systems::body_part_damage::BodyPartDamageSystem;
use serde_json::json;

/// Helper: creates a humanoid body with torso → left arm → left hand, each with hp/max_hp.
fn humanoid_body() -> serde_json::Value {
    json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "hp": 50.0,
                "max_hp": 50.0,
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "hp": 25.0,
                        "max_hp": 25.0,
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
                                "hp": 10.0,
                                "max_hp": 10.0,
                                "temperature": 34.0,
                                "ideal_temperature": 36.5,
                                "insulation": 0.5,
                                "heat_loss": 0.3,
                                "children": [],
                                "equipped": []
                            }
                        ],
                        "equipped": []
                    }
                ],
                "equipped": []
            }
        ]
    })
}

/// Helper: registers the BodyPartDamageSystem on the world.
fn register_system(world: &mut engine_core::ecs::world::World) {
    world.register_system(BodyPartDamageSystem);
}

/// AC004: Targeted damage → correct part hp reduced, status updated.
#[test]
fn test_targeted_damage_reduces_part_hp() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Apply 5 damage to "left hand" (hp: 10 → 5)
    world.damage_entity_part(eid, "left hand", 5.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["hp"], json!(5.0));
    assert_eq!(hand["status"], json!("wounded")); // 5/10 = 0.5 → wounded
}

/// AC005: Untargeted damage → proportional distribution across parts.
#[test]
fn test_untargeted_damage_proportional_distribution() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Total max_hp = 50 + 25 + 10 = 85
    // Apply 85 damage untargeted — each part loses hp proportional to its max_hp share
    world.damage_entity(eid, 85.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let torso = &body["parts"][0];
    let arm = &body["parts"][0]["children"][0];
    let hand = &body["parts"][0]["children"][0]["children"][0];

    // torso: 50/85 * 85 = 50 → hp 0 → broken
    assert_eq!(torso["hp"], json!(0.0));
    assert_eq!(torso["status"], json!("broken"));
    // arm: 25/85 * 85 = 25 → hp 0 → broken
    assert_eq!(arm["hp"], json!(0.0));
    assert_eq!(arm["status"], json!("broken"));
    // hand: 10/85 * 85 = 10 → hp 0 → broken
    assert_eq!(hand["hp"], json!(0.0));
    assert_eq!(hand["status"], json!("broken"));
}

/// AC006: Part hp reaches 0 → broken, then missing on subsequent damage.
#[test]
fn test_part_status_transitions() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Damage left hand to 0 (10 → broken)
    world.damage_entity_part(eid, "left hand", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["status"], json!("broken"));
    assert_eq!(hand["hp"], json!(0.0));

    // Now damage the broken part — should transition to missing
    world.damage_entity_part(eid, "left hand", 1.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["status"], json!("missing"));
}

/// AC007: Entity Health.current is recomputed as sum of part HP after damage processing.
#[test]
fn test_health_recomputed_from_parts() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Damage left hand by 5 (10 → 5)
    world.damage_entity_part(eid, "left hand", 5.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let health = world.get_component(eid, "Health").unwrap();
    // 50 + 25 + 5 = 80
    assert_eq!(health["current"], json!(80.0));
}

/// AC008: Entity without Body → damage_entity falls back to direct Health subtraction.
#[test]
fn test_damage_without_body_fallback() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world
        .set_component(eid, "Health", json!({"current": 50.0, "max": 100.0}))
        .unwrap();

    // No Body component — damage_entity should directly subtract from Health
    world.damage_entity(eid, 15.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let health = world.get_component(eid, "Health").unwrap();
    assert_eq!(health["current"], json!(35.0));
}

/// AC009: Multiple pending damages in one tick are all processed in a single pass.
#[test]
fn test_multiple_damages_single_pass() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Queue two targeted damages to left hand: 3 + 4 = 7 total (10 → 3)
    world.damage_entity_part(eid, "left hand", 3.0);
    world.damage_entity_part(eid, "left hand", 4.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["hp"], json!(3.0));
    assert_eq!(hand["status"], json!("wounded")); // 3/10 = 0.3 → wounded
}

/// AC010: damage_entity_part targets only the specified part.
#[test]
fn test_damage_entity_part_targets_only_specified() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "torso", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let torso = &body["parts"][0];
    let arm = &body["parts"][0]["children"][0];
    let hand = &body["parts"][0]["children"][0]["children"][0];

    // Only torso should be damaged
    assert_eq!(torso["hp"], json!(40.0));
    assert_eq!(arm["hp"], json!(25.0));
    assert_eq!(hand["hp"], json!(10.0));
}

/// AC011: BodyEquipmentSyncSystem auto-unequips items on parts that become wounded/broken.
#[test]
fn test_equipment_sync_after_damage() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    world.register_system(BodyPartDamageSystem);
    world.register_system(BodyEquipmentSyncSystem);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Create equipment with a left hand slot
    let equipment = json!({
        "slots": { "left hand": "gold_glove" }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    // Sync to set equipped on the body
    world.run_system("BodyEquipmentSyncSystem").unwrap();

    // Verify glove is equipped
    let body = world.get_component(eid, "Body").unwrap();
    let hand_eq = &body["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(hand_eq.as_array().unwrap().len(), 1);

    // Damage left hand to broken (10 → 0)
    world.damage_entity_part(eid, "left hand", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    // Sync again — should auto-unequip the glove
    world.run_system("BodyEquipmentSyncSystem").unwrap();

    let body_after = world.get_component(eid, "Body").unwrap();
    let hand_eq_after = &body_after["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(hand_eq_after.as_array().unwrap().len(), 0);

    let equip_after = world.get_component(eid, "Equipment").unwrap();
    assert!(equip_after["slots"]["left hand"].is_null());
}

/// Edge case: damage to a missing part is ignored.
#[test]
fn test_damage_to_missing_part_ignored() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Break the hand
    world.damage_entity_part(eid, "left hand", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    // Make it missing
    world.damage_entity_part(eid, "left hand", 1.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["status"], json!("missing"));

    // Damage the missing part — should be a no-op
    world.damage_entity_part(eid, "left hand", 5.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["status"], json!("missing"));
    assert_eq!(hand["hp"], json!(0.0));
}

/// PendingDamage component is removed after processing.
#[test]
fn test_pending_damage_removed_after_processing() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "left hand", 5.0);
    assert!(world.has_component(eid, "PendingDamage"));

    world.run_system("BodyPartDamageSystem").unwrap();

    assert!(!world.has_component(eid, "PendingDamage"));
}

/// Empty damages array is a no-op.
#[test]
fn test_empty_damages_array_noop() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    // Manually insert an empty PendingDamage
    world
        .components
        .entry("PendingDamage".to_string())
        .or_default()
        .insert(eid, json!({"damages": []}));

    world.run_system("BodyPartDamageSystem").unwrap();

    // Body unchanged, PendingDamage removed
    assert!(!world.has_component(eid, "PendingDamage"));
    let body = world.get_component(eid, "Body").unwrap();
    assert_eq!(body["parts"][0]["hp"], json!(50.0));
}
