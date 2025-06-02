use crate::presentation::ui::factory::{UI_FACTORY, WIDGET_REGISTRY, WidgetProps};
use crate::presentation::ui::schema_loader::load_ui_from_json;
use crate::presentation::ui::widget::UiWidget;
use mlua::{
    Function as LuaFunction, Lua, LuaSerdeExt, Result as LuaResult, Table, Value as LuaValue,
};

pub fn register_ui_api(lua: &Lua, globals: &Table) -> LuaResult<()> {
    crate::presentation::ui::register_all_widgets();

    let globals_table = lua.globals();
    if globals_table
        .get::<Option<Table>>("_ui_callbacks")?
        .is_none()
    {
        let cb_table = lua.create_table()?;
        globals_table.set("_ui_callbacks", cb_table)?;
    }

    let ui = lua.create_table()?;

    ui.set(
        "create_widget",
        lua.create_function(|_, (type_name, props): (String, Table)| {
            let mut rust_props = WidgetProps::new();
            for pair in props.pairs::<String, LuaValue>() {
                let (k, v) = pair?;
                rust_props.insert(k, serde_json::to_value(v).unwrap());
            }
            let widget = UI_FACTORY
                .lock()
                .borrow()
                .create_widget(&type_name, rust_props);
            if let Some(widget) = widget {
                let id = widget.id();
                let binding = WIDGET_REGISTRY.lock();
                let mut registry = binding.borrow_mut();
                registry.insert(id, widget);
                Ok(id)
            } else {
                Ok(0)
            }
        })?,
    )?;

    ui.set(
        "load_json",
        lua.create_function(|_, json_str: String| {
            let mut ids = Vec::new();
            if let Some(root_widget) = load_ui_from_json(&json_str) {
                fn collect_ids(widget: &(dyn UiWidget + Send), ids: &mut Vec<u64>) {
                    ids.push(widget.id());
                }
                collect_ids(root_widget.as_ref(), &mut ids);
                let binding = WIDGET_REGISTRY.lock();
                let mut registry = binding.borrow_mut();
                registry.insert(root_widget.id(), root_widget);
            }
            Ok(ids)
        })?,
    )?;

    ui.set(
        "remove_widget",
        lua.create_function(|_, widget_id: u64| {
            let binding = WIDGET_REGISTRY.lock();
            let mut registry = binding.borrow_mut();
            let removed = registry.remove(&widget_id).is_some();
            Ok(removed)
        })?,
    )?;

    ui.set(
        "set_widget_props",
        lua.create_function(|_, (widget_id, props): (u64, Table)| {
            let mut rust_props = WidgetProps::new();
            for pair in props.pairs::<String, LuaValue>() {
                let (k, v) = pair?;
                rust_props.insert(k, serde_json::to_value(v).unwrap());
            }
            let binding = WIDGET_REGISTRY.lock();
            let mut registry = binding.borrow_mut();
            if let Some(widget) = registry.get_mut(&widget_id) {
                widget.set_props(&rust_props);
                Ok(true)
            } else {
                Ok(false)
            }
        })?,
    )?;

    ui.set(
        "get_widget_props",
        lua.create_function(|lua, widget_id: u64| {
            let binding = WIDGET_REGISTRY.lock();
            let registry = binding.borrow();
            if let Some(widget) = registry.get(&widget_id) {
                macro_rules! try_downcast {
                    ($ty:ty) => {
                        if let Some(w) = widget.as_any().downcast_ref::<$ty>() {
                            let json = serde_json::to_value(w).unwrap();
                            let value = lua.to_value(&json)?;
                            if let LuaValue::Table(table) = value {
                                return Ok(Some(table));
                            } else {
                                return Ok(None);
                            }
                        }
                    };
                }
                try_downcast!(crate::presentation::ui::widget::button::Button);
                try_downcast!(crate::presentation::ui::widget::label::Label);
                try_downcast!(crate::presentation::ui::widget::panel::Panel);
                try_downcast!(crate::presentation::ui::widget::grid_layout::GridLayout);
                try_downcast!(crate::presentation::ui::widget::checkbox::Checkbox);
                try_downcast!(crate::presentation::ui::widget::dropdown::Dropdown);
                try_downcast!(crate::presentation::ui::widget::text_input::TextInput);
                try_downcast!(crate::presentation::ui::widget::context_menu::ContextMenu);
                Ok(None)
            } else {
                Ok(None)
            }
        })?,
    )?;

    ui.set(
        "add_child",
        lua.create_function(|_, (parent_id, child_id): (u64, u64)| {
            let binding = WIDGET_REGISTRY.lock();
            let mut registry = binding.borrow_mut();
            let child = registry.remove(&child_id);
            if let (Some(parent), Some(child)) = (registry.get_mut(&parent_id), child) {
                parent.add_child(child);
                Ok(true)
            } else {
                Ok(false)
            }
        })?,
    )?;

    ui.set(
        "get_children",
        lua.create_function(|_, widget_id: u64| {
            let binding = WIDGET_REGISTRY.lock();
            let registry = binding.borrow();
            if let Some(widget) = registry.get(&widget_id) {
                Ok(widget.get_children())
            } else {
                Ok(Vec::<u64>::new())
            }
        })?,
    )?;

    let _ = ui.set(
        "remove_child",
        lua.create_function(|_, (parent_id, child_id): (u64, u64)| {
            let binding = WIDGET_REGISTRY.lock();
            let mut registry = binding.borrow_mut();
            let mut removed_child: Option<Box<dyn UiWidget + Send>> = None;

            if let Some(parent) = registry.get_mut(&parent_id) {
                if let Some(panel) = parent
                    .as_any_mut()
                    .downcast_mut::<crate::presentation::ui::widget::panel::Panel>(
                ) {
                    if let Some(pos) = panel.children.iter().position(|c| c.id() == child_id) {
                        removed_child = Some(panel.children.remove(pos));
                    }
                }
                if removed_child.is_none() {
                    if let Some(grid) = parent
                        .as_any_mut()
                        .downcast_mut::<crate::presentation::ui::widget::grid_layout::GridLayout>(
                    ) {
                        if let Some(pos) = grid.children.iter().position(|c| c.id() == child_id) {
                            removed_child = Some(grid.children.remove(pos));
                        }
                    }
                }
            }

            if let Some(child) = removed_child {
                registry.insert(child.id(), child);
                Ok(true)
            } else {
                Ok(false)
            }
        })?,
    );

    ui.set(
        "set_callback",
        lua.create_function(
            |lua, (widget_id, event_name, func): (u64, String, LuaFunction)| {
                let callbacks: Table = lua.globals().get::<Table>("_ui_callbacks")?;
                let key = format!("{}_{}", widget_id, event_name);
                callbacks.set(key, func)?;
                Ok(true)
            },
        )?,
    )?;

    ui.set(
        "remove_callback",
        lua.create_function(|lua, (widget_id, event_name): (u64, String)| {
            let callbacks: Table = lua.globals().get::<Table>("_ui_callbacks")?;
            let key = format!("{}_{}", widget_id, event_name);
            callbacks.set(key, mlua::Value::Nil)?;
            Ok(true)
        })?,
    )?;

    ui.set(
        "focus_widget",
        lua.create_function(|_, widget_id: u64| {
            let binding = WIDGET_REGISTRY.lock();
            let mut registry = binding.borrow_mut();
            if let Some(widget) = registry.get_mut(&widget_id) {
                widget.set_focused(true);
                Ok(true)
            } else {
                Ok(false)
            }
        })?,
    )?;

    ui.set(
        "trigger_event",
        lua.create_function(
            move |lua, (widget_id, event_name, args): (u64, String, Table)| {
                let binding = WIDGET_REGISTRY.lock();
                let mut registry = binding.borrow_mut();
                use crate::presentation::ui::UiEvent;
                let event = match event_name.as_str() {
                    "click" => {
                        let x = args.get::<i32>("x").unwrap_or(0);
                        let y = args.get::<i32>("y").unwrap_or(0);
                        UiEvent::Click { x, y }
                    }
                    "key_press" => {
                        let key = args.get::<String>("key").unwrap_or_default();
                        UiEvent::KeyPress { key }
                    }
                    _ => return Ok(false),
                };
                if let Some(widget) = registry.get_mut(&widget_id) {
                    widget.handle_event(&event);
                    let callbacks: Table = lua.globals().get::<Table>("_ui_callbacks")?;
                    let key = format!("{}_{}", widget_id, event_name);
                    if let Some(cb) = callbacks.get::<Option<LuaFunction>>(key)? {
                        let _ = cb.call::<()>((widget_id,));
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )?,
    )?;

    ui.set("send_ui_event", ui.get::<LuaFunction>("trigger_event")?)?;

    globals.set("ui", ui)?;
    Ok(())
}
