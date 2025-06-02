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
pub struct Dropdown {
    pub id: WidgetId,
    pub pos: (i32, i32),
    pub width: usize,
    pub options: Vec<String>,
    pub color: RenderColor,
    pub expanded: bool,
    pub selected: Option<String>,
    pub focused: bool,
    #[serde(skip)]
    pub callbacks: HashMap<String, WidgetCallback>,
    #[serde(skip)]
    pub on_select: Option<Box<dyn FnMut(String) + Send>>,
    pub z_order: i32,
    pub parent: Option<WidgetId>,
}

impl Clone for Dropdown {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            pos: self.pos,
            width: self.width,
            options: self.options.clone(),
            color: self.color,
            expanded: self.expanded,
            selected: self.selected.clone(),
            focused: self.focused,
            callbacks: self.callbacks.clone(),
            on_select: None,
            z_order: self.z_order,
            parent: self.parent,
        }
    }
}

impl Dropdown {
    pub fn new(pos: (i32, i32), width: usize, options: Vec<String>, color: RenderColor) -> Self {
        static mut NEXT_ID: WidgetId = 4000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            pos,
            width,
            options,
            color,
            expanded: false,
            selected: None,
            focused: false,
            callbacks: HashMap::new(),
            on_select: None,
            z_order: 0,
            parent: None,
        }
    }

    pub fn set_on_select(&mut self, cb: Box<dyn FnMut(String) + Send>) {
        self.on_select = Some(cb);
    }
}

impl UiWidget for Dropdown {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        // Render header with < and >, current selection
        renderer.queue_draw(RenderCommand {
            glyph: '<',
            color: self.color,
            pos: self.pos,
        });
        let display = self.selected.clone().unwrap_or_default();
        for (i, ch) in display.chars().enumerate() {
            renderer.queue_draw(RenderCommand {
                glyph: ch,
                color: self.color,
                pos: (self.pos.0 + 1 + i as i32, self.pos.1),
            });
        }
        renderer.queue_draw(RenderCommand {
            glyph: '>',
            color: self.color,
            pos: (self.pos.0 + self.width as i32 - 1, self.pos.1),
        });

        // Render options if expanded
        if self.expanded {
            for (i, opt) in self.options.iter().enumerate() {
                for (j, ch) in opt.chars().enumerate() {
                    renderer.queue_draw(RenderCommand {
                        glyph: ch,
                        color: self.color,
                        pos: (self.pos.0 + j as i32, self.pos.1 + 1 + i as i32),
                    });
                }
            }
        }
    }

    fn handle_event(&mut self, event: &UiEvent) {
        if let UiEvent::Click { x, y } = *event {
            if y == self.pos.1 && x >= self.pos.0 && x < self.pos.0 + self.width as i32 {
                self.expanded = !self.expanded;
            } else if self.expanded {
                let mut selected_option: Option<String> = None;
                for (i, opt) in self.options.iter().enumerate() {
                    if y == self.pos.1 + 1 + i as i32
                        && x >= self.pos.0
                        && x < self.pos.0 + opt.len() as i32
                    {
                        selected_option = Some(opt.clone());
                        break;
                    }
                }
                if let Some(opt) = selected_option {
                    self.selected = Some(opt.clone());
                    if let Some(cb) = self.on_select.as_mut() {
                        cb(opt.clone());
                    }
                    if let Some(cb) = self.callbacks.get("select").cloned() {
                        cb(self);
                    }
                    self.expanded = false;
                }
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
        "Dropdown"
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

pub fn register_dropdown_widget() {
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
        let options = props
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
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
        Box::new(Dropdown::new(pos, width, options, color)) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("Dropdown", Box::new(ctor));
}
