#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::body_equipment_sync::BodyEquipmentSyncSystem;
use engine_core::systems::body_part_damage::BodyPartDamageSystem;
use engine_core::systems::equipment_effect_aggregation::EquipmentEffectAggregationSystem;
use engine_core::systems::stat_calculation::StatCalculationSystem;
use serde_json::json;

// === body_schema tests ===

#[test]
fn test_register_body_schema_and_assign_body_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    let eid = world.spawn_entity();
    let body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
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
    });
    assert!(world.set_component(eid, "Body", body.clone()).is_ok());

    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["name"], "torso");
    assert_eq!(stored["parts"][0]["children"][0]["name"], "left arm");
    assert_eq!(
        stored["parts"][0]["children"][0]["children"][0]["name"],
        "left hand"
    );
}

#[test]
fn test_update_body_part_status_and_equip_item() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    let eid = world.spawn_entity();
    let mut body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [],
                        "equipped": []
                    }
                ],
                "equipped": []
            }
        ]
    });
    world.set_component(eid, "Body", body.clone()).unwrap();

    body["parts"][0]["children"][0]["status"] = json!("wounded");
    world.set_component(eid, "Body", body.clone()).unwrap();
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["children"][0]["status"], "wounded");

    body["parts"][0]["children"][0]["equipped"] = json!(["gold ring"]);
    world.set_component(eid, "Body", body.clone()).unwrap();
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(
        stored["parts"][0]["children"][0]["equipped"][0],
        "gold ring"
    );
}

#[test]
fn test_set_and_query_body_part_temperature_and_insulation() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    let eid = world.spawn_entity();
    let body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
                                "temperature": 34.0,
                                "ideal_temperature": 36.5,
                                "insulation": 0.5,
                                "heat_loss": 0.3,
                                "children": [],
                                "equipped": []
                            }
                        ],
                        "equipped": ["wool glove"]
                    }
                ],
                "equipped": []
            }
        ]
    });
    assert!(world.set_component(eid, "Body", body.clone()).is_ok());

    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["temperature"], 36.5);
    assert_eq!(stored["parts"][0]["children"][0]["insulation"], 1.0);
    assert_eq!(
        stored["parts"][0]["children"][0]["equipped"][0],
        "wool glove"
    );
}

// === body_part_damage tests ===

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

fn register_body_part_damage_system(world: &mut engine_core::ecs::world::World) {
    world.register_system(BodyPartDamageSystem);
}

/// AC004: Targeted damage → correct part hp reduced, status updated.
#[test]
fn test_targeted_damage_reduces_part_hp() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "left hand", 5.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["hp"], json!(5.0));
    assert_eq!(hand["status"], json!("wounded"));
}

/// AC005: Untargeted damage → proportional distribution across parts.
#[test]
fn test_untargeted_damage_proportional_distribution() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity(eid, 85.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let torso = &body["parts"][0];
    let arm = &body["parts"][0]["children"][0];
    let hand = &body["parts"][0]["children"][0]["children"][0];

    assert_eq!(torso["hp"], json!(0.0));
    assert_eq!(torso["status"], json!("broken"));
    assert_eq!(arm["hp"], json!(0.0));
    assert_eq!(arm["status"], json!("broken"));
    assert_eq!(hand["hp"], json!(0.0));
    assert_eq!(hand["status"], json!("broken"));
}

/// AC006: Part hp reaches 0 → broken, then missing on subsequent damage.
#[test]
fn test_part_status_transitions() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "left hand", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["status"], json!("broken"));
    assert_eq!(hand["hp"], json!(0.0));

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
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "left hand", 5.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let health = world.get_component(eid, "Health").unwrap();
    assert_eq!(health["current"], json!(80.0));
}

/// AC008: Entity without Body → damage_entity falls back to direct Health subtraction.
#[test]
fn test_damage_without_body_fallback() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world
        .set_component(eid, "Health", json!({"current": 50.0, "max": 100.0}))
        .unwrap();

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
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "left hand", 3.0);
    world.damage_entity_part(eid, "left hand", 4.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["hp"], json!(3.0));
    assert_eq!(hand["status"], json!("wounded"));
}

/// AC010: damage_entity_part targets only the specified part.
#[test]
fn test_damage_entity_part_targets_only_specified() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();
    register_body_part_damage_system(&mut world);

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

    let equipment = json!({
        "slots": { "left hand": "gold_glove" }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    world.run_system("BodyEquipmentSyncSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand_eq = &body["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(hand_eq.as_array().unwrap().len(), 1);

    world.damage_entity_part(eid, "left hand", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

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
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world.damage_entity_part(eid, "left hand", 10.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    world.damage_entity_part(eid, "left hand", 1.0);
    world.run_system("BodyPartDamageSystem").unwrap();

    let body = world.get_component(eid, "Body").unwrap();
    let hand = &body["parts"][0]["children"][0]["children"][0];
    assert_eq!(hand["status"], json!("missing"));

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
    register_body_part_damage_system(&mut world);

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
    register_body_part_damage_system(&mut world);

    let eid = world.spawn_entity();
    world.set_component(eid, "Body", humanoid_body()).unwrap();
    world
        .set_component(eid, "Health", json!({"current": 85.0, "max": 85.0}))
        .unwrap();

    world
        .components
        .entry("PendingDamage".to_string())
        .or_default()
        .insert(eid, json!({"damages": []}));

    world.run_system("BodyPartDamageSystem").unwrap();

    assert!(!world.has_component(eid, "PendingDamage"));
    let body = world.get_component(eid, "Body").unwrap();
    assert_eq!(body["parts"][0]["hp"], json!(50.0));
}

// === body_equipment_integration tests ===

#[test]
fn test_body_equipment_sync_enforcement() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    world.register_system(BodyEquipmentSyncSystem);
    world.register_system(EquipmentEffectAggregationSystem);
    world.register_system(StatCalculationSystem);

    let ring_id = world.spawn_entity();
    let ring = json!({
        "id": "gold_ring",
        "name": "Gold Ring",
        "slot": "left hand",
        "effects": { "dexterity": 2 }
    });
    world.set_component(ring_id, "Item", ring).unwrap();

    let eid = world.spawn_entity();
    world.set_component(eid, "BaseStats", json!({})).unwrap();
    let body = json!({
        "parts": [
            {
                "name": "torso",
                "kind": "torso",
                "status": "healthy",
                "temperature": 36.5,
                "ideal_temperature": 36.5,
                "insulation": 2.0,
                "heat_loss": 0.1,
                "children": [
                    {
                        "name": "left arm",
                        "kind": "arm",
                        "status": "healthy",
                        "temperature": 35.0,
                        "ideal_temperature": 36.5,
                        "insulation": 1.0,
                        "heat_loss": 0.2,
                        "children": [
                            {
                                "name": "left hand",
                                "kind": "hand",
                                "status": "healthy",
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
    });
    world.set_component(eid, "Body", body).unwrap();

    let equipment = json!({
        "slots": {
            "left hand": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    let mut updated_equipment = equipment.clone();
    updated_equipment["slots"]["left hand"] = json!("gold_ring");
    world
        .set_component(eid, "Equipment", updated_equipment.clone())
        .unwrap();

    world.run_system("BodyEquipmentSyncSystem").unwrap();

    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();

    let body_after = world.get_component(eid, "Body").unwrap();
    let left_hand_equipped = &body_after["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(left_hand_equipped[0], "gold_ring");

    let mut wounded_body = body_after.clone();
    wounded_body["parts"][0]["children"][0]["children"][0]["status"] = json!("wounded");
    world
        .set_component(eid, "Body", wounded_body.clone())
        .unwrap();

    world.run_system("BodyEquipmentSyncSystem").unwrap();

    let equipment_after = world.get_component(eid, "Equipment").unwrap().clone();
    assert!(equipment_after["slots"]["left hand"].is_null());
    let body_after_wound = world.get_component(eid, "Body").unwrap().clone();
    let left_hand_equipped =
        &body_after_wound["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(left_hand_equipped.as_array().unwrap().len(), 0);

    let mut healed_body = body_after_wound.clone();
    healed_body["parts"][0]["children"][0]["children"][0]["status"] = json!("healthy");
    world
        .set_component(eid, "Body", healed_body.clone())
        .unwrap();

    let mut re_equipment = equipment_after.clone();
    re_equipment["slots"]["left hand"] = json!("gold_ring");
    world
        .set_component(eid, "Equipment", re_equipment.clone())
        .unwrap();

    world.run_system("BodyEquipmentSyncSystem").unwrap();

    let body_re_equipped = world.get_component(eid, "Body").unwrap();
    let left_hand_reeq = &body_re_equipped["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(left_hand_reeq[0], "gold_ring");

    world
        .run_system("EquipmentEffectAggregationSystem")
        .unwrap();
    world.run_system("StatCalculationSystem").unwrap();

    let default_stats = json!({});
    let stats = world.get_component(eid, "Stats").unwrap_or(&default_stats);
    let dex = stats
        .get("dexterity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    assert_eq!(dex, 2.0);
}
