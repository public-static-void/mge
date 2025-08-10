use crate::presentation::renderer::PresentationRenderer;
use crate::presentation::ui::widget::widget_trait::{UiWidget, WidgetId};
use std::any::Any;

/// A grid of widgets with a focused widget.
pub struct FocusGrid {
    id: WidgetId,
    children: Vec<(Box<dyn UiWidget + Send>, i32, i32)>, // (widget, col, row)
    focused: Option<usize>,
}

impl Clone for FocusGrid {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            children: self
                .children
                .iter()
                .map(|(w, c, r)| (w.boxed_clone(), *c, *r))
                .collect(),
            focused: self.focused,
        }
    }
}

impl FocusGrid {
    /// Create a new FocusGrid with the given number of columns and rows.
    pub fn new(_cols: i32, _rows: i32) -> Self {
        static mut NEXT_ID: WidgetId = 2_000_000;
        let id = unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        };
        Self {
            id,
            children: Vec::new(),
            focused: None,
        }
    }

    /// Add a child to the grid.
    pub fn add_child(&mut self, widget: Box<dyn UiWidget + Send>, col: i32, row: i32) {
        self.children.push((widget, col, row));
    }

    /// Move the focus to a child.
    pub fn move_focus_public(&mut self, dcol: i32, drow: i32) {
        if self.children.is_empty() {
            self.focused = None;
            return;
        }
        let (cur_col, cur_row) = if let Some(idx) = self.focused {
            (self.children[idx].1, self.children[idx].2)
        } else {
            self.focused = Some(0);
            return;
        };

        let mut best_idx = None;
        let mut best_dist = i32::MAX;

        for (i, &(_, col, row)) in self.children.iter().enumerate() {
            if i == self.focused.unwrap() {
                continue;
            }
            if dcol != 0 && (row != cur_row) {
                continue;
            }
            if drow != 0 && (col != cur_col) {
                continue;
            }
            if dcol != 0 && (col - cur_col).signum() != dcol.signum() {
                continue;
            }
            if drow != 0 && (row - cur_row).signum() != drow.signum() {
                continue;
            }
            let dist = (col - cur_col).abs() + (row - cur_row).abs();
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx {
            self.focused = Some(idx);
        }
    }

    /// Get the focused index.
    pub fn focused_index(&self) -> Option<usize> {
        self.focused
    }
}

impl Default for FocusGrid {
    fn default() -> Self {
        Self::new(1, 1)
    }
}

impl UiWidget for FocusGrid {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        for (child, _, _) in &mut self.children {
            child.render(renderer);
        }
    }

    fn handle_event(&mut self, event: &crate::presentation::ui::UiEvent) {
        if let Some(idx) = self.focused {
            self.children[idx].0.handle_event(event);
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
        (0, 0)
    }
    fn focus_group(&self) -> Option<u32> {
        None
    }

    fn set_callback(
        &mut self,
        _event: &str,
        _cb: Option<std::sync::Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>>,
    ) {
        // No-op: FocusGrid does not support generic callbacks.
    }

    fn add_child(&mut self, child: Box<dyn UiWidget + Send>) {
        // By default, add at (0, 0).
        self.children.push((child, 0, 0));
    }

    fn widget_type(&self) -> &'static str {
        "FocusGrid"
    }

    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(self.clone())
    }
}
