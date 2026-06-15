use engine_core::presentation::ui::register_all_widgets;
use engine_core::presentation::ui::schema_loader::load_ui_from_json;

#[test]
fn test_load_button_from_json() {
    register_all_widgets();
    let json = r#"{
        "type": "Button",
        "props": {
            "label": "Click Me",
            "pos": [10, 5],
            "color": [255, 0, 0]
        }
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_some());
    assert_eq!(widget.unwrap().widget_type(), "Button");
}

#[test]
fn test_load_label_from_json() {
    register_all_widgets();
    let json = r#"{
        "type": "Label",
        "props": {
            "text": "Hello World",
            "pos": [0, 0]
        }
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_some());
    assert_eq!(widget.unwrap().widget_type(), "Label");
}

#[test]
fn test_load_panel_with_children() {
    register_all_widgets();
    let json = r#"{
        "type": "Panel",
        "props": {
            "pos": [0, 0]
        },
        "children": [
            {
                "type": "Label",
                "props": {
                    "text": "Child A"
                }
            },
            {
                "type": "Button",
                "props": {
                    "label": "Child B"
                }
            }
        ]
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_some());
    let w = widget.unwrap();
    assert_eq!(w.widget_type(), "Panel");
    let children = w.get_children();
    assert_eq!(children.len(), 2);
}

#[test]
fn test_load_missing_type_returns_none() {
    register_all_widgets();
    let json = r#"{
        "props": { "label": "No Type" }
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_none());
}

#[test]
fn test_load_unregistered_type_returns_none() {
    register_all_widgets();
    let json = r#"{
        "type": "NonExistentWidget"
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_none());
}

#[test]
fn test_load_invalid_json_returns_none() {
    register_all_widgets();
    let widget = load_ui_from_json("not valid json");
    assert!(widget.is_none());
}

#[test]
fn test_load_empty_json_object_returns_none() {
    register_all_widgets();
    let widget = load_ui_from_json("{}");
    assert!(widget.is_none());
}

#[test]
fn test_load_null_json_returns_none() {
    register_all_widgets();
    let widget = load_ui_from_json("null");
    assert!(widget.is_none());
}

#[test]
fn test_load_panel_with_multiple_children() {
    register_all_widgets();
    let json = r#"{
        "type": "Panel",
        "props": {
            "pos": [0, 0]
        },
        "children": [
            { "type": "Label", "props": { "text": "First" } },
            { "type": "Label", "props": { "text": "Second" } },
            { "type": "Label", "props": { "text": "Third" } }
        ]
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_some());
    let w = widget.unwrap();
    assert_eq!(w.get_children().len(), 3);
}

#[test]
fn test_load_button_with_props() {
    register_all_widgets();
    let json = r#"{
        "type": "Button",
        "props": {
            "label": "Props Test",
            "pos": [5, 10],
            "color": [0, 255, 0]
        }
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_some());
    let w = widget.unwrap();
    let any = w.as_any();
    let btn = any.downcast_ref::<engine_core::presentation::ui::widget::Button>();
    assert!(btn.is_some());
    assert_eq!(btn.unwrap().label, "Props Test");
}

#[test]
fn test_load_type_with_non_string_type_is_none() {
    register_all_widgets();
    let json = r#"{
        "type": 123
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_none());
}

#[test]
fn test_load_empty_children_ok() {
    register_all_widgets();
    let json = r#"{
        "type": "Panel",
        "children": []
    }"#;
    let widget = load_ui_from_json(json);
    assert!(widget.is_some());
    assert_eq!(widget.unwrap().get_children().len(), 0);
}
