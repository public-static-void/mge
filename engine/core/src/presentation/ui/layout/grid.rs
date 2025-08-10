use crate::presentation::renderer::PresentationRenderer;
use crate::presentation::ui::widget::widget_trait::{
    SetPos, UiWidget, WidgetId, update_struct_from_props,
};
use crate::presentation::ui::{Alignment, Padding};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// A grid layout
#[derive(Serialize, Deserialize)]
pub struct GridLayout {
    /// The ID of the widget
    pub id: WidgetId,
    /// The children
    #[serde(skip)]
    pub children: Vec<Box<dyn UiWidget + Send>>,
    /// The number of columns
    pub columns: usize,
    /// The cell size
    pub cell_size: (i32, i32),
    /// The spacing
    pub spacing: (i32, i32),
    /// The origin
    pub origin: (i32, i32),
    /// The alignment
    pub alignment: Alignment,
    /// The padding
    pub padding: Padding,
    /// The z-order
    pub z_order: i32,
    /// The parent
    pub parent: Option<WidgetId>,
}

impl Clone for GridLayout {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            children: self.children.iter().map(|c| c.boxed_clone()).collect(),
            columns: self.columns,
            cell_size: self.cell_size,
            spacing: self.spacing,
            origin: self.origin,
            alignment: self.alignment,
            padding: self.padding,
            z_order: self.z_order,
            parent: self.parent,
        }
    }
}

impl GridLayout {
    /// Create a new grid layout
    pub fn new(cell_size: (i32, i32), spacing: (i32, i32)) -> Self {
        static mut NEXT_ID: WidgetId = 1_000_000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            children: Vec::new(),
            columns: 1,
            cell_size,
            spacing,
            origin: (0, 0),
            alignment: Alignment::Start,
            padding: Padding::uniform(0),
            z_order: 0,
            parent: None,
        }
    }

    /// Set the number of columns
    pub fn set_columns(&mut self, columns: usize) {
        self.columns = columns.max(1);
    }

    /// Set the origin
    pub fn set_origin(&mut self, origin: (i32, i32)) {
        self.origin = origin;
    }

    /// Set the alignment
    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = alignment;
    }

    /// Set the padding
    pub fn set_padding(&mut self, padding: Padding) {
        self.padding = padding;
    }

    /// Add a child
    pub fn add_child(&mut self, mut widget: Box<dyn UiWidget + Send>) {
        widget.set_parent(Some(self.id));
        self.children.push(widget);
    }

    fn grid_size(&self) -> (usize, usize) {
        let cols = self.columns;
        let len = self.children.len();
        let rows = len.div_ceil(cols);
        (cols, rows)
    }

    fn grid_area(&self) -> (i32, i32) {
        let (cols, rows) = self.grid_size();
        let (cell_w, cell_h) = self.cell_size;
        let (space_x, space_y) = self.spacing;
        (
            (cols as i32) * cell_w + ((cols - 1) as i32) * space_x,
            (rows as i32) * cell_h + ((rows - 1) as i32) * space_y,
        )
    }

    fn widget_area(&self) -> (i32, i32) {
        let (grid_w, grid_h) = self.grid_area();
        (
            self.padding.left + grid_w + self.padding.right,
            self.padding.top + grid_h + self.padding.bottom,
        )
    }

    fn alignment_offset(&self) -> (i32, i32) {
        let (widget_w, widget_h) = self.widget_area();
        let (grid_w, grid_h) = self.grid_area();
        let offset_x = match self.alignment {
            Alignment::Start | Alignment::End => 0,
            Alignment::Center => (widget_w - grid_w) / 2,
        };
        let offset_y = match self.alignment {
            Alignment::Start | Alignment::End => 0,
            Alignment::Center => (widget_h - grid_h) / 2,
        };
        (offset_x, offset_y)
    }

    fn child_position(&self, index: usize) -> (i32, i32) {
        let (cols, _) = self.grid_size();
        let (cell_w, cell_h) = self.cell_size;
        let (space_x, space_y) = self.spacing;
        let col = (index % cols) as i32;
        let row = (index / cols) as i32;
        let (offset_x, offset_y) = self.alignment_offset();
        let x = self.origin.0 + self.padding.left + offset_x + col * (cell_w + space_x);
        let y = self.origin.1 + self.padding.top + offset_y + row * (cell_h + space_y);
        (x, y)
    }

    fn child_at(&self, x: i32, y: i32) -> Option<usize> {
        let (cols, rows) = self.grid_size();
        let (cell_w, cell_h) = self.cell_size;
        let (space_x, space_y) = self.spacing;
        let (offset_x, offset_y) = self.alignment_offset();

        let rel_x = x - self.origin.0 - self.padding.left - offset_x;
        let rel_y = y - self.origin.1 - self.padding.top - offset_y;

        let col = rel_x / (cell_w + space_x);
        let row = rel_y / (cell_h + space_y);

        if col < 0 || row < 0 || col as usize >= cols || row as usize >= rows {
            return None;
        }

        let index = row as usize * cols + col as usize;
        if index >= self.children.len() {
            return None;
        }

        let cell_x = col * (cell_w + space_x);
        let cell_y = row * (cell_h + space_y);
        if rel_x < cell_x || rel_x >= cell_x + cell_w || rel_y < cell_y || rel_y >= cell_y + cell_h
        {
            return None;
        }

        Some(index)
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new((1, 1), (0, 0))
    }
}

impl UiWidget for GridLayout {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        let positions: Vec<(i32, i32)> = (0..self.children.len())
            .map(|i| self.child_position(i))
            .collect();

        for (child, &(x, y)) in self.children.iter_mut().zip(&positions) {
            if let Some(label) = child
                .as_any_mut()
                .downcast_mut::<crate::presentation::ui::widget::label::Label>()
            {
                label.set_pos((x, y));
            } else if let Some(button) = child
                .as_any_mut()
                .downcast_mut::<crate::presentation::ui::widget::button::Button>(
            ) {
                button.set_pos((x, y));
            } else if let Some(text_input) = child
                .as_any_mut()
                .downcast_mut::<crate::presentation::ui::widget::text_input::TextInput>(
            ) {
                text_input.set_pos((x, y));
            }
            child.render(renderer);
        }
    }

    fn handle_event(&mut self, event: &crate::presentation::ui::UiEvent) {
        let positions: Vec<(i32, i32)> = (0..self.children.len())
            .map(|i| self.child_position(i))
            .collect();
        for (child, &(x, y)) in self.children.iter_mut().zip(&positions) {
            if let Some(label) = child
                .as_any_mut()
                .downcast_mut::<crate::presentation::ui::widget::label::Label>()
            {
                label.set_pos((x, y));
            } else if let Some(button) = child
                .as_any_mut()
                .downcast_mut::<crate::presentation::ui::widget::button::Button>(
            ) {
                button.set_pos((x, y));
            } else if let Some(text_input) = child
                .as_any_mut()
                .downcast_mut::<crate::presentation::ui::widget::text_input::TextInput>(
            ) {
                text_input.set_pos((x, y));
            }
        }
        match event {
            crate::presentation::ui::UiEvent::Click { x, y } => {
                if let Some(idx) = self.child_at(*x, *y) {
                    self.children[idx].handle_event(event);
                }
            }
            _ => {
                for child in &mut self.children {
                    child.handle_event(event);
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
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn is_focused(&self) -> bool {
        false
    }

    fn focus_pos(&self) -> (i32, i32) {
        self.origin
    }

    fn focus_group(&self) -> Option<u32> {
        None
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
        "GridLayout"
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

/// Register the grid layout widget
pub fn register_grid_layout_widget() {
    use crate::presentation::ui::factory::{UI_FACTORY, WidgetProps};

    let ctor = |props: WidgetProps| {
        let cell_size = props
            .get("cell_size")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                if arr.len() == 2 {
                    Some((
                        arr[0].as_i64().unwrap_or(1) as i32,
                        arr[1].as_i64().unwrap_or(1) as i32,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or((1, 1));
        let spacing = props
            .get("spacing")
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
        let columns = props
            .get("columns")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

        let mut layout = GridLayout::new(cell_size, spacing);
        layout.set_columns(columns);

        Box::new(layout) as Box<dyn UiWidget + Send>
    };
    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("GridLayout", Box::new(ctor));
}
