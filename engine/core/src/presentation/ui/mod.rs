//! User interface module
//!
//! The user interface module contains all the widgets and their implementations.
//! The user interface is the main entry point to the UI library.

/// UI Events
pub mod event;
/// UI factory
pub mod factory;
/// UI layout
pub mod layout;
/// UI root node
pub mod root;
/// UI schema loader
pub mod schema_loader;
/// UI widgets
pub mod widget;

pub use event::UiEvent;
pub use factory::*;
pub use layout::direction::*;
pub use layout::linear::*;
pub use root::*;
pub use schema_loader::*;
pub use widget::Button;
pub use widget::Checkbox;
pub use widget::ContextMenu;
pub use widget::Dropdown;
pub use widget::EventLogWidget;
pub use widget::Label;
pub use widget::TextInput;
pub use widget::UiNode;
pub use widget::UiWidget;
pub use widget::WidgetId;

/// Register all widgets
pub fn register_all_widgets() {
    widget::button::register_button_widget();
    widget::label::register_label_widget();
    widget::checkbox::register_checkbox_widget();
    widget::dropdown::register_dropdown_widget();
    widget::text_input::register_text_input_widget();
    widget::context_menu::register_context_menu_widget();
    widget::panel::register_panel_widget();
}
