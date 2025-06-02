use crate::presentation::ui::factory::{UI_FACTORY, WidgetProps};
use crate::presentation::ui::widget::UiWidget;
use serde_json::Value;

/// Loads a UI tree from a JSON string and returns the root widget.
/// Returns None if parsing or construction fails.
pub fn load_ui_from_json(json: &str) -> Option<Box<dyn UiWidget + Send>> {
    let value: Value = serde_json::from_str(json).ok()?;
    build_widget_from_value(&value)
}

fn build_widget_from_value(value: &Value) -> Option<Box<dyn UiWidget + Send>> {
    let type_name = value.get("type")?.as_str()?;
    let props = value
        .get("props")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut widget_props = WidgetProps::new();
    for (k, v) in props {
        widget_props.insert(k, v);
    }
    let mut widget = UI_FACTORY
        .lock()
        .borrow()
        .create_widget(type_name, widget_props)?;
    if let Some(children) = value.get("children").and_then(|v| v.as_array()) {
        for child in children {
            if let Some(child_widget) = build_widget_from_value(child) {
                widget.add_child(child_widget);
            }
        }
    }
    Some(widget)
}
