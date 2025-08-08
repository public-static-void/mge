use crate::presentation::renderer::{PresentationRenderer, RenderColor, RenderCommand};
use crate::presentation::ui::UiEvent;
use crate::presentation::ui::widget::widget_trait::{UiWidget, WidgetId, update_struct_from_props};
use serde::{Deserialize, Serialize};
use std::any::Any;

pub type ContextMenuAction = Box<dyn FnMut() + Send>;

#[derive(Serialize, Deserialize)]
pub struct ContextMenuEntry {
    pub label: String,
    pub enabled: bool,
    #[serde(skip)]
    pub action: Option<ContextMenuAction>,
    #[serde(skip)]
    pub submenu: Option<Box<ContextMenu>>,
}

impl Clone for ContextMenuEntry {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            enabled: self.enabled,
            action: None, // Callbacks are not cloneable
            submenu: self.submenu.as_ref().map(|s| Box::new((**s).clone())),
        }
    }
}

impl ContextMenuEntry {
    pub fn new<L: Into<String>>(
        label: L,
        enabled: bool,
        action: Option<ContextMenuAction>,
    ) -> Self {
        Self {
            label: label.into(),
            enabled,
            action,
            submenu: None,
        }
    }

    pub fn with_submenu<L: Into<String>>(label: L, submenu: ContextMenu) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            action: None,
            submenu: Some(Box::new(submenu)),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ContextMenu {
    pub id: WidgetId,
    pub pos: (i32, i32),
    pub entries: Vec<ContextMenuEntry>,
    pub selected: usize,
    pub open_submenu: Option<usize>,
    pub color: RenderColor,
    pub bg_color: RenderColor,
    pub visible: bool,
    pub parent: Option<WidgetId>,
}

impl Clone for ContextMenu {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            pos: self.pos,
            entries: self.entries.to_vec(),
            selected: self.selected,
            open_submenu: self.open_submenu,
            color: self.color,
            bg_color: self.bg_color,
            visible: self.visible,
            parent: self.parent,
        }
    }
}

impl ContextMenu {
    pub fn new(
        pos: (i32, i32),
        entries: Vec<ContextMenuEntry>,
        color: RenderColor,
        bg_color: RenderColor,
    ) -> Self {
        static mut NEXT_ID: WidgetId = 800_000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            pos,
            entries,
            selected: 0,
            open_submenu: None,
            color,
            bg_color,
            visible: false,
            parent: None,
        }
    }

    pub fn show(&mut self, pos: (i32, i32)) {
        self.pos = pos;
        self.visible = true;
        self.selected = 0;
        self.open_submenu = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.open_submenu = None;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn set_parent(&mut self, parent: WidgetId) {
        self.parent = Some(parent);
    }

    fn entry_rect(&self, idx: usize) -> (i32, i32, i32, i32) {
        // (x, y, w, h)
        (self.pos.0, self.pos.1 + idx as i32, self.width() as i32, 1)
    }

    fn width(&self) -> usize {
        self.entries
            .iter()
            .map(|e| e.label.len() + if e.submenu.is_some() { 2 } else { 0 })
            .max()
            .unwrap_or(8)
            + 2
    }
}

impl UiWidget for ContextMenu {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        if !self.visible {
            return;
        }
        let width = self.width();
        for (i, entry) in self.entries.iter().enumerate() {
            let y = self.pos.1 + i as i32;
            for x in 0..width {
                renderer.queue_draw(RenderCommand {
                    glyph: ' ',
                    color: self.bg_color,
                    pos: (self.pos.0 + x as i32, y),
                });
            }
            if i == self.selected {
                for x in 0..width {
                    renderer.queue_draw(RenderCommand {
                        glyph: ' ',
                        color: RenderColor(50, 50, 150),
                        pos: (self.pos.0 + x as i32, y),
                    });
                }
            }
            for (j, ch) in entry.label.chars().enumerate() {
                renderer.queue_draw(RenderCommand {
                    glyph: ch,
                    color: if entry.enabled {
                        self.color
                    } else {
                        RenderColor(120, 120, 120)
                    },
                    pos: (self.pos.0 + 1 + j as i32, y),
                });
            }
            if entry.submenu.is_some() {
                renderer.queue_draw(RenderCommand {
                    glyph: '▶',
                    color: self.color,
                    pos: (self.pos.0 + width as i32 - 2, y),
                });
            }
        }
        if let Some(sub_idx) = self.open_submenu
            && let Some(submenu) = self.entries[sub_idx].submenu.as_mut()
        {
            submenu.render(renderer);
        }
    }

    fn handle_event(&mut self, event: &UiEvent) {
        if !self.visible {
            return;
        }
        match event {
            UiEvent::Click { x, y } => {
                for (i, _) in self.entries.iter().enumerate() {
                    let (_x, entry_y, width, _h) = self.entry_rect(i);
                    if *y == entry_y && *x >= self.pos.0 && *x < self.pos.0 + width {
                        self.selected = i;
                        if self.entries[i].enabled {
                            let menu_width = self.width() as i32;
                            let menu_selected = self.selected;
                            if let Some(ref mut submenu) = self.entries[menu_selected].submenu {
                                self.open_submenu = Some(menu_selected);
                                submenu.show((
                                    self.pos.0 + menu_width,
                                    self.pos.1 + menu_selected as i32,
                                ));
                            }
                        }
                        return;
                    }
                }
                // Click outside menu closes it
                self.hide();
            }
            UiEvent::KeyPress { key } => match key.as_str() {
                "Up" => {
                    let mut idx = self.selected;
                    loop {
                        if idx == 0 {
                            idx = self.entries.len() - 1;
                        } else {
                            idx -= 1;
                        }
                        if self.entries[idx].enabled {
                            self.selected = idx;
                            self.open_submenu = None;
                            break;
                        }
                        if idx == self.selected {
                            break;
                        }
                    }
                }
                "Down" => {
                    let mut idx = self.selected;
                    loop {
                        idx = (idx + 1) % self.entries.len();
                        if self.entries[idx].enabled {
                            self.selected = idx;
                            self.open_submenu = None;
                            break;
                        }
                        if idx == self.selected {
                            break;
                        }
                    }
                }
                "Left" => {
                    if let Some(_parent_id) = self.parent {
                        self.hide();
                    }
                }
                "Right" => {
                    let menu_width = self.width() as i32;
                    let menu_selected = self.selected;
                    if let Some(ref mut submenu) = self.entries[menu_selected].submenu {
                        self.open_submenu = Some(menu_selected);
                        submenu.show((self.pos.0 + menu_width, self.pos.1 + menu_selected as i32));
                    }
                }
                "Enter" | "Space" => {
                    if self.entries[self.selected].enabled {
                        let menu_width = self.width() as i32;
                        let menu_selected = self.selected;
                        if let Some(ref mut submenu) = self.entries[menu_selected].submenu {
                            self.open_submenu = Some(menu_selected);
                            submenu
                                .show((self.pos.0 + menu_width, self.pos.1 + menu_selected as i32));
                        } else if let Some(ref mut action) = self.entries[self.selected].action {
                            action();
                            self.hide();
                        }
                    }
                }
                "Esc" => {
                    self.hide();
                }
                _ => {}
            },
        }
        // Forward event to open submenu if any
        if let Some(sub_idx) = self.open_submenu
            && let Some(submenu) = self.entries[sub_idx].submenu.as_mut()
        {
            submenu.handle_event(event);
        }
    }

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
        // No-op: ContextMenu does not support generic callbacks.
    }
    fn set_props(&mut self, props: &std::collections::HashMap<String, serde_json::Value>) {
        update_struct_from_props(self, props);
    }

    fn widget_type(&self) -> &'static str {
        "ContextMenu"
    }

    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(self.clone())
    }
}

// --- Registration function for data-driven UI ---
pub fn register_context_menu_widget() {
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
        let bg_color = props
            .get("bg_color")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                if arr.len() == 3 {
                    Some(RenderColor(
                        arr[0].as_u64().unwrap_or(0) as u8,
                        arr[1].as_u64().unwrap_or(0) as u8,
                        arr[2].as_u64().unwrap_or(0) as u8,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(RenderColor(0, 0, 0));
        let entries = props
            .get("entries")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|entry| entry.get("label").and_then(|l| l.as_str()))
                    .map(|label| ContextMenuEntry::new(label, true, None))
                    .collect::<Vec<ContextMenuEntry>>()
            })
            .unwrap_or_default();

        Box::new(ContextMenu::new(pos, entries, color, bg_color)) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("ContextMenu", Box::new(ctor));
}
