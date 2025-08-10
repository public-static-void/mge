use super::super::widget::{Button, Label, UiNode, UiWidget};
use super::direction::{Alignment, LayoutDirection, Padding};
use crate::presentation::renderer::{PresentationRenderer, RenderCommand};
use crate::presentation::ui::UiEvent;
use std::any::Any;

/// A linear layout widget.
pub struct Layout {
    id: u64,
    /// The direction of the layout
    pub direction: LayoutDirection,
    /// The position of the layout
    pub pos: (i32, i32),
    /// The spacing between items
    pub spacing: i32,
    /// The alignment of items
    pub alignment: Alignment,
    /// The padding of the layout
    pub padding: Padding,
    /// The children of the layout
    pub children: Vec<UiNode>,
}

impl Clone for Layout {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            direction: self.direction,
            pos: self.pos,
            spacing: self.spacing,
            alignment: self.alignment,
            padding: self.padding,
            children: self.children.iter().map(|c| c.boxed_clone()).collect(),
        }
    }
}

impl Layout {
    /// Creates a new linear layout
    pub fn new(direction: LayoutDirection, pos: (i32, i32), spacing: i32) -> Self {
        static mut NEXT_ID: u64 = 200_000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            direction,
            pos,
            spacing,
            alignment: Alignment::Start,
            padding: Padding::uniform(0),
            children: Vec::new(),
        }
    }

    /// Adds a child to the layout
    pub fn add_child(&mut self, child: UiNode) {
        self.children.push(child);
    }

    /// Sets the alignment of the layout
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
    }

    /// Sets the padding of the layout
    pub fn set_padding(&mut self, padding: Padding) {
        self.padding = padding;
    }
}

impl UiWidget for Layout {
    fn id(&self) -> u64 {
        self.id
    }
    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        let mut offset = 0;
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

            let (dx, dy) = match self.direction {
                LayoutDirection::Row => (self.pos.0 + offset, self.pos.1),
                LayoutDirection::Column => (self.pos.0, self.pos.1 + offset),
            };

            let child_size = match self.direction {
                LayoutDirection::Row => {
                    if let Some(label) = child.as_any().downcast_ref::<Label>() {
                        label.text.chars().count() as i32
                    } else if let Some(button) = child.as_any().downcast_ref::<Button>() {
                        button.label.chars().count() as i32
                    } else {
                        1
                    }
                }
                LayoutDirection::Column => 1,
            };

            let total_space = match self.direction {
                LayoutDirection::Row => self.padding.left + child_size + self.padding.right,
                LayoutDirection::Column => self.padding.top + 1 + self.padding.bottom,
            };

            let align_offset = match self.alignment {
                Alignment::Start => 0,
                Alignment::Center => (total_space - child_size) / 2,
                Alignment::End => total_space - child_size,
            };

            let final_x = match self.direction {
                LayoutDirection::Row => dx + self.padding.left + align_offset,
                LayoutDirection::Column => dx + self.padding.left,
            };
            let final_y = match self.direction {
                LayoutDirection::Row => dy + self.padding.top,
                LayoutDirection::Column => dy + self.padding.top + align_offset,
            };

            let mut offset_renderer = OffsetRenderer {
                base: renderer,
                dx: final_x,
                dy: final_y,
            };
            child.render(&mut offset_renderer);
            offset += child_size + self.spacing + self.padding.left + self.padding.right;
        }
    }

    fn handle_event(&mut self, event: &UiEvent) {
        let mut offset = 0;
        for child in self.children.iter_mut() {
            let (dx, dy) = match self.direction {
                LayoutDirection::Row => (self.pos.0 + offset, self.pos.1),
                LayoutDirection::Column => (self.pos.0, self.pos.1 + offset),
            };
            let adjusted_event = match *event {
                UiEvent::Click { x, y } => UiEvent::Click {
                    x: x - dx,
                    y: y - dy,
                },
                UiEvent::KeyPress { ref key } => UiEvent::KeyPress { key: key.clone() },
            };
            child.handle_event(&adjusted_event);

            let child_size = match self.direction {
                LayoutDirection::Row => {
                    if let Some(label) = child.as_any().downcast_ref::<Label>() {
                        label.text.chars().count() as i32
                    } else if let Some(button) = child.as_any().downcast_ref::<Button>() {
                        button.label.chars().count() as i32
                    } else {
                        1
                    }
                }
                LayoutDirection::Column => 1,
            };
            offset += child_size + self.spacing + self.padding.left + self.padding.right;
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
        // No-op: Layout does not support callbacks.
    }

    fn add_child(&mut self, child: Box<dyn UiWidget + Send>) {
        self.children.push(UiNode::from(child));
    }

    fn widget_type(&self) -> &'static str {
        "Layout"
    }

    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(self.clone())
    }
}
