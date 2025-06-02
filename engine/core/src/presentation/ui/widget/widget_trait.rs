use crate::presentation::renderer::PresentationRenderer;
use crate::presentation::ui::UiEvent;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::any::Any;

/// Unique identifier for a UI widget.
pub type WidgetId = u64;

/// Callback type for widget events.
pub type WidgetCallback = std::sync::Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>;

pub trait UiWidget: Send {
    fn id(&self) -> WidgetId;
    fn render(&mut self, renderer: &mut dyn PresentationRenderer);
    fn handle_event(&mut self, event: &UiEvent);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn is_focusable(&self) -> bool {
        false
    }
    fn set_focused(&mut self, _focused: bool) {}
    fn is_focused(&self) -> bool {
        false
    }
    fn focus_pos(&self) -> (i32, i32) {
        (0, 0)
    }
    fn focus_group(&self) -> Option<u32> {
        None
    }

    fn set_callback(&mut self, _event: &str, _cb: Option<WidgetCallback>) {}

    fn add_child(&mut self, _child: Box<dyn UiWidget + Send>) {}

    fn get_children(&self) -> Vec<u64> {
        Vec::new()
    }

    fn set_props(&mut self, _props: &std::collections::HashMap<String, serde_json::Value>) {}

    fn widget_type(&self) -> &'static str;

    fn get_parent(&self) -> Option<WidgetId> {
        None
    }
    fn set_parent(&mut self, _parent: Option<WidgetId>) {}

    fn set_z_order(&mut self, _z: i32) {}
    fn get_z_order(&self) -> i32 {
        0
    }

    /// Clone this widget as a boxed trait object.
    fn boxed_clone(&self) -> Box<dyn UiWidget + Send>;
}

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

pub trait SetPos {
    fn set_pos(&mut self, pos: (i32, i32));
}
