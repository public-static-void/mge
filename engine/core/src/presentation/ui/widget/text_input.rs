use crate::presentation::renderer::{PresentationRenderer, RenderColor, RenderCommand};
use crate::presentation::ui::UiEvent;
use crate::presentation::ui::widget::widget_trait::{
    SetPos, UiWidget, WidgetId, update_struct_from_props,
};
use serde::{Deserialize, Serialize};
use std::any::Any;

pub type OnTextChange = Box<dyn FnMut(&str) + Send>;

#[derive(Serialize, Deserialize)]
pub struct TextInput {
    pub id: WidgetId,
    pub pos: (i32, i32),
    pub width: usize,
    pub text: String,
    pub cursor: usize,
    pub color: RenderColor,
    pub focused: bool,
    #[serde(skip)]
    pub on_change: Option<OnTextChange>,
    pub group: Option<u32>,
}

impl Clone for TextInput {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            pos: self.pos,
            width: self.width,
            text: self.text.clone(),
            cursor: self.cursor,
            color: self.color,
            focused: self.focused,
            on_change: None,
            group: self.group,
        }
    }
}

impl TextInput {
    pub fn new(pos: (i32, i32), width: usize, color: RenderColor, group: Option<u32>) -> Self {
        static mut NEXT_ID: WidgetId = 500_000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            pos,
            width,
            text: String::new(),
            cursor: 0,
            color,
            focused: false,
            on_change: None,
            group,
        }
    }

    pub fn set_on_change(&mut self, f: Box<dyn FnMut(&str) + Send>) {
        self.on_change = Some(f);
    }

    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.text.len();
        if let Some(cb) = self.on_change.as_mut() {
            cb(&self.text);
        }
    }
    pub fn is_focused(&self) -> bool {
        self.focused
    }
}

impl UiWidget for TextInput {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        for i in 0..self.width {
            renderer.queue_draw(RenderCommand {
                glyph: if i < self.text.len() {
                    self.text.chars().nth(i).unwrap()
                } else {
                    '_'
                },
                color: self.color,
                pos: (self.pos.0 + i as i32, self.pos.1),
            });
        }
        if self.focused && self.cursor <= self.width {
            renderer.queue_draw(RenderCommand {
                glyph: '|',
                color: RenderColor(255, 255, 255),
                pos: (self.pos.0 + self.cursor as i32, self.pos.1),
            });
        }
    }

    fn handle_event(&mut self, event: &UiEvent) {
        if let UiEvent::Click { x, y } = *event {
            if y == self.pos.1 && x >= self.pos.0 && x < self.pos.0 + self.width as i32 {
                self.focused = true;
                self.cursor = (x - self.pos.0) as usize;
                if self.cursor > self.text.len() {
                    self.cursor = self.text.len();
                }
            } else {
                self.focused = false;
            }
        }
        if let UiEvent::KeyPress { .. } = *event
            && self.focused {
                // Optionally implement text editing here
            }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn is_focusable(&self) -> bool {
        true
    }
    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }
    fn is_focused(&self) -> bool {
        self.focused
    }
    fn focus_pos(&self) -> (i32, i32) {
        self.pos
    }
    fn focus_group(&self) -> Option<u32> {
        self.group
    }

    fn set_callback(
        &mut self,
        _event: &str,
        _cb: Option<std::sync::Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>>,
    ) {
        // No-op: TextInput does not support generic callbacks.
    }
    fn set_props(&mut self, props: &std::collections::HashMap<String, serde_json::Value>) {
        update_struct_from_props(self, props);
    }

    fn widget_type(&self) -> &'static str {
        "TextInput"
    }

    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(self.clone())
    }
}

impl SetPos for TextInput {
    fn set_pos(&mut self, pos: (i32, i32)) {
        self.pos = pos;
    }
}

// --- Registration function for data-driven UI ---
pub fn register_text_input_widget() {
    use crate::presentation::renderer::RenderColor;
    use crate::presentation::ui::factory::{UI_FACTORY, WidgetProps};

    let ctor = |props: WidgetProps| {
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
        let width = props
            .get("width")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(10);
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
        let group = props
            .get("group")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        Box::new(TextInput::new(pos, width, color, group)) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("TextInput", Box::new(ctor));
}
