#[test]
fn can_register_body_schema_and_assign_body_component() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let body_schema_json = include_str!("../../assets/schemas/body.json");
    registry
        .register_external_schema_from_json(body_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Create a hierarchical body: torso -> left arm -> left hand
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

    // Query the body component and check the nested structure
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["name"], "torso");
    assert_eq!(stored["parts"][0]["children"][0]["name"], "left arm");
    assert_eq!(
        stored["parts"][0]["children"][0]["children"][0]["name"],
        "left hand"
    );
}

#[test]
fn can_update_body_part_status_and_equip_item() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let body_schema_json = include_str!("../../assets/schemas/body.json");
    registry
        .register_external_schema_from_json(body_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    world.current_mode = "roguelike".to_string();

    // Add a simple body (all required fields present)
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

    // Wound the left arm
    body["parts"][0]["children"][0]["status"] = json!("wounded");
    world.set_component(eid, "Body", body.clone()).unwrap();
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(stored["parts"][0]["children"][0]["status"], "wounded");

    // Equip a ring on the left arm
    body["parts"][0]["children"][0]["equipped"] = json!(["gold ring"]);
    world.set_component(eid, "Body", body.clone()).unwrap();
    let stored = world.get_component(eid, "Body").unwrap();
    assert_eq!(
        stored["parts"][0]["children"][0]["equipped"][0],
        "gold ring"
    );
}

#[test]
fn can_set_and_query_body_part_temperature_and_insulation() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::world::World;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    let mut registry = ComponentRegistry::new();
    let body_schema_json = include_str!("../../assets/schemas/body.json");
    registry
        .register_external_schema_from_json(body_schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
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
