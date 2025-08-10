use crate::presentation::renderer::{PresentationRenderer, RenderColor, RenderCommand};
use crate::presentation::ui::UiEvent;
use crate::presentation::ui::factory::{UI_FACTORY, WidgetProps};
use crate::presentation::ui::widget::widget_trait::{
    SetPos, UiWidget, WidgetId, update_struct_from_props,
};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// A label widget.
#[derive(Serialize, Deserialize, Clone)]
pub struct Label {
    /// The widget ID
    pub id: WidgetId,
    /// The label text
    pub text: String,
    /// The position
    pub pos: (i32, i32),
    /// The color
    pub color: RenderColor,
}

impl Label {
    /// Create a new label
    pub fn new<T: Into<String>>(text: T, pos: (i32, i32), color: RenderColor) -> Self {
        static mut NEXT_ID: WidgetId = 1;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            text: text.into(),
            pos,
            color,
        }
    }
}

impl SetPos for Label {
    fn set_pos(&mut self, pos: (i32, i32)) {
        self.pos = pos;
    }
}

impl UiWidget for Label {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        for (i, ch) in self.text.chars().enumerate() {
            renderer.queue_draw(RenderCommand {
                glyph: ch,
                color: self.color,
                pos: (self.pos.0 + i as i32, self.pos.1),
            });
        }
    }
    fn handle_event(&mut self, _event: &UiEvent) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn set_callback(
        &mut self,
        _event: &str,
        _cb: Option<std::sync::Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>>,
    ) {
        // Labels do not support callbacks/events.
    }
    fn set_props(&mut self, props: &std::collections::HashMap<String, serde_json::Value>) {
        update_struct_from_props(self, props);
    }

    fn widget_type(&self) -> &'static str {
        "Label"
    }

    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(self.clone())
    }
}

/// Register the label widget
/// Factory registration for data-driven UI
pub fn register_label_widget() {
    let ctor = |props: WidgetProps| {
        let text = props.get("text").and_then(|v| v.as_str()).unwrap_or("");
        let pos = props
            .get("pos")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                if arr.len() == 2 {
                    Some((
                        arr[0].as_i64().unwrap_or(0) as i32,
                        arr[1].as_i64().unwrap_or(0) as i32,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or((0, 0));
        let color = props
            .get("color")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                if arr.len() == 3 {
                    Some(RenderColor(
                        arr[0].as_u64().unwrap_or(255) as u8,
                        arr[1].as_u64().unwrap_or(255) as u8,
                        arr[2].as_u64().unwrap_or(255) as u8,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(RenderColor(255, 255, 255));
        Box::new(Label::new(text, pos, color)) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("Label", Box::new(ctor));
}
