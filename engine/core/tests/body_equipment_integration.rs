#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::body_equipment_sync::BodyEquipmentSyncSystem;
use serde_json::json;

#[test]
fn test_body_equipment_sync_enforcement() {
    // Use the shared helper to load all schemas via config and create world
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    // Register the integration system
    world.register_system(BodyEquipmentSyncSystem);

    // Create an item (ring) with a stat effect
    let ring_id = world.spawn_entity();
    let ring = json!({
        "id": "gold_ring",
        "name": "Gold Ring",
        "slot": "left hand",
        "effects": { "dexterity": 2 }
    });
    world.set_component(ring_id, "Item", ring).unwrap();

    // Create a body: torso -> left arm -> left hand
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
    world.set_component(eid, "Body", body).unwrap();

    // Create equipment with left_hand slot
    let equipment = json!({
        "slots": {
            "left hand": null
        }
    });
    world
        .set_component(eid, "Equipment", equipment.clone())
        .unwrap();

    // Equip the ring via Equipment
    let mut updated_equipment = equipment.clone();
    updated_equipment["slots"]["left hand"] = json!("gold_ring");
    world
        .set_component(eid, "Equipment", updated_equipment.clone())
        .unwrap();

    // Run the sync system
    world.run_system("BodyEquipmentSyncSystem", None).unwrap();

    // The ring should now appear in Body.parts[*].equipped for "left hand"
    let body_after = world.get_component(eid, "Body").unwrap();
    let left_hand_equipped = &body_after["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(left_hand_equipped[0], "gold_ring");

    // Wound the left hand
    let mut wounded_body = body_after.clone();
    wounded_body["parts"][0]["children"][0]["children"][0]["status"] = json!("wounded");
    world
        .set_component(eid, "Body", wounded_body.clone())
        .unwrap();

    // Run the sync system again
    world.run_system("BodyEquipmentSyncSystem", None).unwrap();

    // The ring should be auto-unequipped from both Equipment and Body
    let equipment_after = world.get_component(eid, "Equipment").unwrap();
    assert!(equipment_after["slots"]["left hand"].is_null());
    let body_after_wound = world.get_component(eid, "Body").unwrap();
    let left_hand_equipped =
        &body_after_wound["parts"][0]["children"][0]["children"][0]["equipped"];
    assert_eq!(left_hand_equipped.as_array().unwrap().len(), 0);

    // Equip a ring directly to the left hand (Body)
    let mut direct_body = body_after_wound.clone();
    direct_body["parts"][0]["children"][0]["children"][0]["equipped"] = json!(["gold_ring"]);
    world
        .set_component(eid, "Body", direct_body.clone())
        .unwrap();

    // Run the sync system
    world.run_system("BodyEquipmentSyncSystem", None).unwrap();

    // The effect should be aggregated (dexterity +2)
    let default_stats = json!({});
    let stats = world.get_component(eid, "Stats").unwrap_or(&default_stats);
    let dex = stats
        .get("dexterity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    assert_eq!(dex, 2.0);
}
