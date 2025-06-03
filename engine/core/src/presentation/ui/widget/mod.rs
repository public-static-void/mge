pub mod widget_trait;
pub use widget_trait::{UiWidget, WidgetId};

pub mod button;
pub use button::Button;

pub mod checkbox;
pub use checkbox::Checkbox;

pub mod label;
pub use label::Label;

pub mod text_input;
pub use text_input::TextInput;

pub mod dropdown;
pub use dropdown::Dropdown;

pub mod context_menu;
pub use context_menu::ContextMenu;

pub mod panel;
pub use panel::Panel;

pub mod focus_grid;
pub use focus_grid::FocusGrid;

pub mod dynamic;

/// A node in the UI tree: any widget as a boxed trait object.
pub type UiNode = Box<dyn UiWidget + Send>;
