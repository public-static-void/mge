use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use engine_core::presentation::renderer::RenderColor;
use engine_core::presentation::ui::root::{Direction, UiRoot};
use engine_core::presentation::ui::widget::button::Button;
use engine_core::presentation::ui::widget::checkbox::Checkbox;
use engine_core::presentation::ui::widget::focus_grid::FocusGrid;
use engine_core::presentation::ui::widget::text_input::TextInput;
use serde_json::json;
use std::sync::{Arc, Mutex};

// === mode_enforcement tests ===

#[test]
fn test_get_component_mode_enforcement() {
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();

    world.current_mode = "colony".to_string();
    world
        .set_component(id, "MagicPower", json!({ "mana": 42 }))
        .unwrap();
    assert!(
        world.get_component(id, "MagicPower").is_some(),
        "Should be able to get component in allowed mode"
    );

    world.current_mode = "roguelike".to_string();
    assert!(
        world.get_component(id, "MagicPower").is_none(),
        "Should NOT be able to get component in disallowed mode"
    );
}

#[test]
fn test_remove_component_mode_enforcement() {
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();

    world.current_mode = "colony".to_string();
    world
        .set_component(id, "MagicPower", json!({ "mana": 42 }))
        .unwrap();
    assert!(world.remove_component(id, "MagicPower").is_ok());

    world
        .set_component(id, "MagicPower", json!({ "mana": 99 }))
        .unwrap();
    world.current_mode = "roguelike".to_string();
    assert!(world.remove_component(id, "MagicPower").is_err());
}

#[test]
fn test_get_entities_with_component_mode_enforcement() {
    let schema_json = r#"
    {
        "title": "MagicPower",
        "type": "object",
        "properties": { "mana": { "type": "number" } },
        "required": ["mana"],
        "modes": ["colony"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let id = world.spawn_entity();
    world.current_mode = "colony".to_string();
    world
        .set_component(id, "MagicPower", json!({ "mana": 42 }))
        .unwrap();

    let entities = world.get_entities_with_component("MagicPower");
    assert_eq!(entities, vec![id]);

    world.current_mode = "roguelike".to_string();
    let entities = world.get_entities_with_component("MagicPower");
    assert!(entities.is_empty());
}

#[test]
fn test_schema_driven_mode_enforcement() {
    let inventory_schema = r#"
    {
      "title": "Inventory",
      "type": "object",
      "properties": {
        "slots": { "type": "array", "items": { "type": "string" } },
        "weight": { "type": "number" }
      },
      "required": ["slots", "weight"],
      "modes": ["roguelike"]
    }
    "#;

    let mut registry = ComponentRegistry::new();
    registry
        .register_external_schema_from_json(inventory_schema)
        .unwrap();
    let registry = Arc::new(Mutex::new(registry));

    let mut world = World::new(registry.clone());
    let entity = world.spawn_entity();

    world.current_mode = "colony".to_string();
    let result = world.set_component(entity, "Inventory", json!({"slots": [], "weight": 0.0}));
    assert!(
        result.is_err(),
        "Inventory should NOT be allowed in colony mode"
    );

    world.current_mode = "roguelike".to_string();
    let result = world.set_component(entity, "Inventory", json!({"slots": [], "weight": 0.0}));
    assert!(
        result.is_ok(),
        "Inventory should be allowed in roguelike mode"
    );
}

// === focus tests ===

#[test]
fn test_physical_navigation_right_and_down() {
    let mut root = UiRoot::new();

    let button1 = Button::new(
        "A",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );
    let button2 = Button::new(
        "B",
        (10, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );
    let button3 = Button::new(
        "C",
        (0, 10),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        None,
    );

    root.add_child(Box::new(button1));
    root.add_child(Box::new(button2));
    root.add_child(Box::new(button3));

    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));

    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(1));

    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(2));

    root.focus_physical(Direction::Left);
    assert_eq!(root.focused_index(), Some(2));
}

#[test]
fn test_focus_groups_restrict_navigation() {
    let mut root = UiRoot::new();

    let button1 = Button::new(
        "A",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(1),
    );
    let button2 = Button::new(
        "B",
        (10, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(2),
    );
    let input1 = TextInput::new((0, 10), 8, RenderColor(0, 255, 255), Some(1));
    let cb2 = Checkbox::new("C", (10, 10), RenderColor(0, 255, 0), Some(2));

    root.add_child(Box::new(button1));
    root.add_child(Box::new(button2));
    root.add_child(Box::new(input1));
    root.add_child(Box::new(cb2));

    root.set_focus_group(Some(1));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(2));

    root.set_focus_group(Some(2));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(1));
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(3));

    root.set_focus_group(None);
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(1));
    root.focus_physical(Direction::Down);
    assert_eq!(root.focused_index(), Some(3));
}

#[test]
fn test_focus_group_exclusion() {
    let mut root = UiRoot::new();

    let button1 = Button::new(
        "A",
        (0, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(1),
    );
    let button2 = Button::new(
        "B",
        (10, 0),
        RenderColor(0, 255, 0),
        Some(Box::new(|| {})),
        Some(2),
    );

    root.add_child(Box::new(button1));
    root.add_child(Box::new(button2));

    root.set_focus_group(Some(1));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));
    root.focus_physical(Direction::Right);
    assert_eq!(root.focused_index(), Some(0));
}

#[test]
fn test_grid_layout_navigation() {
    let mut grid = FocusGrid::new(2, 2);

    grid.add_child(
        Box::new(Button::new(
            "A",
            (0, 0),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        0,
        0,
    );
    grid.add_child(
        Box::new(Button::new(
            "B",
            (1, 0),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        1,
        0,
    );
    grid.add_child(
        Box::new(Button::new(
            "C",
            (0, 1),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        0,
        1,
    );
    grid.add_child(
        Box::new(Button::new(
            "D",
            (1, 1),
            RenderColor(0, 255, 0),
            Some(Box::new(|| {})),
            None,
        )),
        1,
        1,
    );

    grid.move_focus_public(0, 0);
    assert_eq!(grid.focused_index(), Some(0));

    grid.move_focus_public(1, 0);
    assert_eq!(grid.focused_index(), Some(1));

    grid.move_focus_public(0, 1);
    assert_eq!(grid.focused_index(), Some(3));

    grid.move_focus_public(-1, 0);
    assert_eq!(grid.focused_index(), Some(2));

    grid.move_focus_public(0, -1);
    assert_eq!(grid.focused_index(), Some(0));
}
