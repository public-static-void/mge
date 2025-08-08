use crate::presentation::renderer::{PresentationRenderer, RenderColor, RenderCommand};
use crate::presentation::ui::UiEvent;
use crate::presentation::ui::factory::{UI_FACTORY, WidgetProps};
use crate::presentation::ui::widget::widget_trait::{
    UiWidget, WidgetCallback, WidgetId, update_struct_from_props,
};
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct Checkbox {
    pub id: WidgetId,
    pub checked: bool,
    pub label: String,
    pub pos: (i32, i32),
    pub color: RenderColor,
    pub group: Option<u32>,
    pub focused: bool,
    #[serde(skip)]
    pub callbacks: HashMap<String, WidgetCallback>,
    #[serde(skip)]
    pub on_toggle: Option<Box<dyn FnMut(bool) + Send>>,
    pub z_order: i32,
    pub parent: Option<WidgetId>,
}

impl Clone for Checkbox {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            checked: self.checked,
            label: self.label.clone(),
            pos: self.pos,
            color: self.color,
            group: self.group,
            focused: self.focused,
            callbacks: self.callbacks.clone(),
            on_toggle: None,
            z_order: self.z_order,
            parent: self.parent,
        }
    }
}

impl Checkbox {
    pub fn new(
        label: impl Into<String>,
        pos: (i32, i32),
        color: RenderColor,
        group: Option<u32>,
    ) -> Self {
        static mut NEXT_ID: WidgetId = 3000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            checked: false,
            label: label.into(),
            pos,
            color,
            group,
            focused: false,
            callbacks: HashMap::new(),
            on_toggle: None,
            z_order: 0,
            parent: None,
        }
    }

    pub fn set_on_toggle(&mut self, cb: Box<dyn FnMut(bool) + Send>) {
        self.on_toggle = Some(cb);
    }
}

impl UiWidget for Checkbox {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        let box_char = if self.checked { '☑' } else { '☐' };
        renderer.queue_draw(RenderCommand {
            glyph: box_char,
            color: self.color,
            pos: self.pos,
        });
        for (i, ch) in self.label.chars().enumerate() {
            renderer.queue_draw(RenderCommand {
                glyph: ch,
                color: self.color,
                pos: (self.pos.0 + 2 + i as i32, self.pos.1),
            });
        }
        if self.focused {
            renderer.queue_draw(RenderCommand {
                glyph: '>',
                color: RenderColor(255, 255, 0),
                pos: (self.pos.0 - 2, self.pos.1),
            });
        }
    }

    fn handle_event(&mut self, event: &UiEvent) {
        if let UiEvent::Click { x, y } = *event
            && y == self.pos.1 && x >= self.pos.0 && x < self.pos.0 + self.label.len() as i32 + 2 {
                self.checked = !self.checked;
                if let Some(cb) = self.on_toggle.as_mut() {
                    cb(self.checked);
                }
                if let Some(cb) = self.callbacks.get("toggle").cloned() {
                    cb(self);
                }
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
        event: &str,
        cb: Option<Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>>,
    ) {
        if let Some(cb) = cb {
            self.callbacks.insert(event.to_string(), cb);
        } else {
            self.callbacks.remove(event);
        }
    }
    fn set_props(&mut self, props: &std::collections::HashMap<String, serde_json::Value>) {
        update_struct_from_props(self, props);
    }
    fn widget_type(&self) -> &'static str {
        "Checkbox"
    }
    fn get_parent(&self) -> Option<WidgetId> {
        self.parent
    }
    fn set_parent(&mut self, parent: Option<WidgetId>) {
        self.parent = parent;
    }
    fn set_z_order(&mut self, z: i32) {
        self.z_order = z;
    }
    fn get_z_order(&self) -> i32 {
        self.z_order
    }

    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(self.clone())
    }
}

pub fn register_checkbox_widget() {
    let ctor = |props: WidgetProps| {
        let label = props
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("Checkbox")
            .to_string();
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
        let group = props
            .get("group")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        Box::new(Checkbox::new(label, pos, color, group)) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("Checkbox", Box::new(ctor));
}
