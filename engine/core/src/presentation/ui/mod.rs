pub mod event;
pub use event::UiEvent;

pub mod factory;
pub mod layout;
pub mod root;
pub mod schema_loader;
pub mod widget;

pub use factory::*;
pub use schema_loader::*;

pub use layout::direction::*;
pub use layout::linear::*;

pub use root::*;

pub use widget::Button;
pub use widget::Checkbox;
pub use widget::ContextMenu;
pub use widget::Dropdown;
pub use widget::Label;
pub use widget::TextInput;
pub use widget::UiNode;
pub use widget::UiWidget;
pub use widget::WidgetId;

/// Registers all UI widgets with the factory.
/// Call this once at engine startup before loading any UI from data or script.
pub fn register_all_widgets() {
    widget::button::register_button_widget();
    widget::label::register_label_widget();
    widget::checkbox::register_checkbox_widget();
    widget::dropdown::register_dropdown_widget();
    widget::text_input::register_text_input_widget();
    widget::context_menu::register_context_menu_widget();
    widget::panel::register_panel_widget();
}
