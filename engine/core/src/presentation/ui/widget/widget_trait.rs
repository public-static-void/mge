use crate::presentation::renderer::PresentationRenderer;
use crate::presentation::ui::UiEvent;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::Any;

/// Unique identifier for a UI widget.
pub type WidgetId = u64;

/// Callback type for widget events.
pub type WidgetCallback = std::sync::Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>;

/// A trait for widgets to implement.
pub trait UiWidget: Send {
    /// Get the widget's ID
    fn id(&self) -> WidgetId;

    /// Render the widget
    fn render(&mut self, renderer: &mut dyn PresentationRenderer);

    /// Handle a UI event
    fn handle_event(&mut self, event: &UiEvent);

    /// Get the widget
    fn as_any(&self) -> &dyn Any;

    /// Get the widget as mutable
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Whether the widget is focusable
    fn is_focusable(&self) -> bool {
        false
    }

    /// Set the widget's focus state
    fn set_focused(&mut self, _focused: bool) {}

    /// Whether the widget is currently focused
    fn is_focused(&self) -> bool {
        false
    }

    /// Get the widget's focus position
    fn focus_pos(&self) -> (i32, i32) {
        (0, 0)
    }

    /// Get the widget's focus group
    fn focus_group(&self) -> Option<u32> {
        None
    }

    /// Set the callback for a given event
    fn set_callback(&mut self, _event: &str, _cb: Option<WidgetCallback>) {}

    /// Add a child
    fn add_child(&mut self, _child: Box<dyn UiWidget + Send>) {}

    /// Get the widget's children
    fn get_children(&self) -> Vec<u64> {
        Vec::new()
    }

    /// Get the widget's props
    fn set_props(&mut self, _props: &std::collections::HashMap<String, serde_json::Value>) {}

    /// Get the widget's type
    fn widget_type(&self) -> &'static str;

    /// Get the widget's parent
    fn get_parent(&self) -> Option<WidgetId> {
        None
    }

    /// Set the widget's parent
    fn set_parent(&mut self, _parent: Option<WidgetId>) {}

    /// Set the widget's z-order
    fn set_z_order(&mut self, _z: i32) {}

    /// Get the widget's z-order
    fn get_z_order(&self) -> i32 {
        0
    }

    /// Clone this widget as a boxed trait object.
    fn boxed_clone(&self) -> Box<dyn UiWidget + Send>;
}

/// Update a widget from props
pub fn update_struct_from_props<T: Serialize + DeserializeOwned>(
    s: &mut T,
    props: &std::collections::HashMap<String, serde_json::Value>,
) {
    let mut value = serde_json::to_value(&*s).expect("Widget must be serializable");
    if let serde_json::Value::Object(ref mut obj) = value {
        for (k, v) in props {
            obj.insert(k.clone(), v.clone());
        }
    }
    *s = serde_json::from_value(value).expect("Widget must be deserializable");
}

/// Set the position of a widget
pub trait SetPos {
    /// Set the widget's position
    fn set_pos(&mut self, pos: (i32, i32));
}
