#[path = "helpers/world.rs"]
mod world_helper;

use serde_json::json;

#[test]
fn test_register_body_schema_and_assign_body_component() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "roguelike".to_string();

    // Create a hierarchical body
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
