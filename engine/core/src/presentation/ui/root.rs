use crate::presentation::ui::UiEvent;
use crate::presentation::ui::widget::UiNode;

pub struct UiRoot {
    pub children: Vec<UiNode>,
    focused: Option<usize>,   // index in children
    focus_group: Option<u32>, // Optional: current focus group id
}

impl UiRoot {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            focused: None,
            focus_group: None,
        }
    }
    pub fn add_child(&mut self, child: UiNode) {
        self.children.push(child);
    }
    pub fn render(
        &mut self,
        renderer: &mut dyn crate::presentation::renderer::PresentationRenderer,
    ) {
        for child in self.children.iter_mut() {
            child.render(renderer);
        }
    }
    pub fn handle_event(&mut self, event: &UiEvent) {
        if let UiEvent::KeyPress { key } = event {
            let direction = match key.as_str() {
                "Tab" | "Right" => Some(Direction::Right),
                "Left" => Some(Direction::Left),
                "Down" => Some(Direction::Down),
                "Up" => Some(Direction::Up),
                _ => None,
            };
            if let Some(dir) = direction {
                self.focus_physical(dir);
                return;
            }
        }
        if let Some(idx) = self.focused {
            if let Some(child) = self.children.get_mut(idx) {
                child.handle_event(event);
                return;
            }
        }
        for child in self.children.iter_mut().rev() {
            child.handle_event(event);
        }
    }

    pub fn focus_physical(&mut self, direction: Direction) {
        let len = self.children.len();
        if len == 0 {
            self.focused = None;
            return;
        }
        // Find all focusable widgets in the current group (or all if no group)
        let group = self.focus_group;
        let focusable: Vec<(usize, (i32, i32))> = self
            .children
            .iter()
            .enumerate()
            .filter(|(_, w)| w.is_focusable() && (group.is_none() || w.focus_group() == group))
            .map(|(i, w)| (i, w.focus_pos()))
            .collect();

        if focusable.is_empty() {
            self.focused = None;
            return;
        }

        // Get current focused widget's position
        let (cur_idx, cur_pos) = if let Some(idx) = self.focused {
            let pos = self.children[idx].focus_pos();
            (idx, pos)
        } else {
            // If nothing focused, pick first in group
            let idx = focusable[0].0;
            self.set_focus(idx);
            return;
        };

        // Find the nearest widget in the given direction
        let mut best_idx = None;
        let mut best_score = f32::MAX;
        for (i, pos) in focusable {
            if i == cur_idx {
                continue;
            }
            if !is_in_direction(cur_pos, pos, direction) {
                continue;
            }
            let score = navigation_score(cur_pos, pos, direction);
            if score < best_score {
                best_score = score;
                best_idx = Some(i);
            }
        }

        if let Some(idx) = best_idx {
            self.set_focus(idx);
        }
    }

    fn set_focus(&mut self, idx: usize) {
        if let Some(prev) = self.focused {
            self.children[prev].set_focused(false);
        }
        self.children[idx].set_focused(true);
        self.focused = Some(idx);
    }

    pub fn set_focus_group(&mut self, group: Option<u32>) {
        self.focus_group = group;
        self.focused = None;
    }

    pub fn clear_focus(&mut self) {
        if let Some(idx) = self.focused.take() {
            self.children[idx].set_focused(false);
        }
    }

    pub fn focused_index(&self) -> Option<usize> {
        self.focused
    }
}

impl Default for UiRoot {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Right,
    Left,
    Down,
    Up,
}

// Returns true if `to` is in the requested direction from `from`
fn is_in_direction(from: (i32, i32), to: (i32, i32), dir: Direction) -> bool {
    match dir {
        Direction::Right => to.0 > from.0,
        Direction::Left => to.0 < from.0,
        Direction::Down => to.1 > from.1,
        Direction::Up => to.1 < from.1,
    }
}

// Lower score = better candidate
fn navigation_score(from: (i32, i32), to: (i32, i32), dir: Direction) -> f32 {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    match dir {
        Direction::Right | Direction::Left => {
            // Prefer horizontal proximity, but penalize vertical distance
            (dx.abs() as f32) + (dy.abs() as f32) * 2.0
        }
        Direction::Down | Direction::Up => (dy.abs() as f32) + (dx.abs() as f32) * 2.0,
    }
}
