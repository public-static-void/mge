use super::widget_trait::{UiWidget, WidgetId, update_struct_from_props};
use crate::presentation::renderer::{PresentationRenderer, RenderCommand};
use crate::presentation::ui::UiEvent;
use serde::{Deserialize, Serialize};
use std::any::Any;

/// A node in the UI tree: any widget as a boxed trait object.
pub type UiNode = Box<dyn UiWidget + Send>;

/// A UI panel.
#[derive(Serialize, Deserialize)]
pub struct Panel {
    /// The widget ID
    pub id: WidgetId,
    /// The position
    pub pos: (i32, i32),
    /// The children
    #[serde(skip)]
    pub children: Vec<UiNode>,
    /// The z-order
    pub z_order: i32,
    /// The parent
    pub parent: Option<WidgetId>,
}

// --- Manual Clone implementation ---
impl Clone for Panel {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            pos: self.pos,
            children: self.children.iter().map(|c| c.boxed_clone()).collect(),
            z_order: self.z_order,
            parent: self.parent,
        }
    }
}

impl Panel {
    /// Create a new panel
    pub fn new(pos: (i32, i32)) -> Self {
        static mut NEXT_ID: WidgetId = 100_000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            pos,
            children: Vec::new(),
            z_order: 0,
            parent: None,
        }
    }
}

impl UiWidget for Panel {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        for child in self.children.iter_mut() {
            struct OffsetRenderer<'a> {
                base: &'a mut dyn PresentationRenderer,
                dx: i32,
                dy: i32,
            }
            impl PresentationRenderer for OffsetRenderer<'_> {
                fn queue_draw(&mut self, mut cmd: RenderCommand) {
                    cmd.pos.0 += self.dx;
                    cmd.pos.1 += self.dy;
                    self.base.queue_draw(cmd);
                }
                fn queue_draw_cell(
                    &mut self,
                    pos: (i32, i32),
                    cell: &crate::map::cell_key::CellKey,
                ) {
                    self.base
                        .queue_draw_cell((pos.0 + self.dx, pos.1 + self.dy), cell);
                }
                fn present(&mut self) {
                    self.base.present();
                }
                fn clear(&mut self) {
                    self.base.clear();
                }
            }
            let mut offset = OffsetRenderer {
                base: renderer,
                dx: self.pos.0,
                dy: self.pos.1,
            };
            child.render(&mut offset);
        }
    }
    fn handle_event(&mut self, event: &UiEvent) {
        for child in self.children.iter_mut() {
            child.handle_event(event);
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
    }

    fn add_child(&mut self, mut child: Box<dyn UiWidget + Send>) {
        child.set_parent(Some(self.id));
        self.children.push(child);
    }
    fn get_children(&self) -> Vec<u64> {
        self.children.iter().map(|c| c.id()).collect()
    }
    fn set_props(&mut self, props: &std::collections::HashMap<String, serde_json::Value>) {
        update_struct_from_props(self, props);
    }
    fn widget_type(&self) -> &'static str {
        "Panel"
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
        // Note: children are also cloned.
        Box::new(self.clone())
    }
}

/// Register the panel
pub fn register_panel_widget() {
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
        Box::new(Panel::new(pos)) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("Panel", Box::new(ctor));
}
