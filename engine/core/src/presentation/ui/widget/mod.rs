//! Widgets for the UI
//!
//! Widgets are the building blocks of the UI. They are used to create complex UIs
//! by combining smaller widgets.

/// Button widget
pub mod button;
/// Checkbox widget
pub mod checkbox;
/// Context menu
pub mod context_menu;
/// Dropdown
pub mod dropdown;
/// Dynamic widget
pub mod dynamic;
/// Event log
pub mod event_log;
/// Focus grid
pub mod focus_grid;
/// Label
pub mod label;
/// Panel
pub mod panel;
/// Text input
pub mod text_input;
/// Widget trait
pub mod widget_trait;

pub use button::Button;
pub use checkbox::Checkbox;
pub use context_menu::ContextMenu;
pub use dropdown::Dropdown;
pub use event_log::EventLogWidget;
pub use focus_grid::FocusGrid;
pub use label::Label;
pub use panel::Panel;
pub use text_input::TextInput;
pub use widget_trait::{UiWidget, WidgetId};

/// A node in the UI tree: any widget as a boxed trait object.
pub type UiNode = Box<dyn UiWidget + Send>;
