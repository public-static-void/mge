use engine_core::presentation::ui::factory::{UI_FACTORY, WIDGET_REGISTRY, WidgetProps};
use engine_core::presentation::ui::schema_loader::load_ui_from_json;
use engine_core::presentation::ui::widget::UiWidget;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use serde_json::Value;
use serde_pyobject::from_pyobject;
use std::sync::Arc;

#[pyclass]
pub struct UiApi {}

#[pymethods]
impl UiApi {
    #[new]
    pub fn new() -> Self {
        engine_core::presentation::ui::register_all_widgets();
        UiApi {}
    }

    pub fn create_widget(&self, type_name: String, props: Bound<'_, PyAny>) -> PyResult<u64> {
        let props_dict: &Bound<PyDict> = props.downcast::<PyDict>()?;
        let mut widget_props = WidgetProps::new();
        for (k, v) in props_dict.iter() {
            let key = k.extract::<String>()?;
            let val: Value = from_pyobject(v)?;
            widget_props.insert(key, val);
        }
        let factory_binding = UI_FACTORY.lock();
        let widget = factory_binding
            .borrow()
            .create_widget(&type_name, widget_props);
        if let Some(widget) = widget {
            let id = widget.id();
            let registry_binding = WIDGET_REGISTRY.lock();
            registry_binding.borrow_mut().insert(id, widget);
            Ok(id)
        } else {
            Ok(0)
        }
    }

    pub fn load_json(&self, json_str: String) -> Vec<u64> {
        let mut ids = Vec::new();
        if let Some(root_widget) = load_ui_from_json(&json_str) {
            ids.push(root_widget.id());
            let registry_binding = WIDGET_REGISTRY.lock();
            registry_binding
                .borrow_mut()
                .insert(root_widget.id(), root_widget);
        }
        ids
    }

    pub fn set_widget_props(&self, widget_id: u64, props: Bound<'_, PyAny>) -> PyResult<bool> {
        let props_dict: &Bound<PyDict> = props.downcast::<PyDict>()?;
        let mut widget_props = WidgetProps::new();
        for (k, v) in props_dict.iter() {
            let key = k.extract::<String>()?;
            let val: Value = from_pyobject(v)?;
            widget_props.insert(key, val);
        }
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(widget) = registry.get_mut(&widget_id) {
            widget.set_props(&widget_props);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_widget_props(&self, py: Python<'_>, widget_id: u64) -> PyResult<Option<PyObject>> {
        let registry_binding = WIDGET_REGISTRY.lock();
        let registry = registry_binding.borrow();
        if let Some(widget) = registry.get(&widget_id) {
            macro_rules! try_downcast {
                ($ty:ty) => {
                    if let Some(w) = widget.as_any().downcast_ref::<$ty>() {
                        let json = serde_json::to_value(w).unwrap();
                        let obj = serde_pyobject::to_pyobject(py, &json)?;
                        if let Ok(dict) = obj.extract::<Bound<PyDict>>() {
                            return Ok(Some(dict.into()));
                        } else {
                            return Ok(Some(obj.into()));
                        }
                    }
                };
            }
            try_downcast!(engine_core::presentation::ui::layout::grid::GridLayout);
            try_downcast!(engine_core::presentation::ui::widget::button::Button);
            try_downcast!(engine_core::presentation::ui::widget::label::Label);
            try_downcast!(engine_core::presentation::ui::widget::panel::Panel);
            try_downcast!(engine_core::presentation::ui::widget::checkbox::Checkbox);
            try_downcast!(engine_core::presentation::ui::widget::dropdown::Dropdown);
            try_downcast!(engine_core::presentation::ui::widget::text_input::TextInput);
            try_downcast!(engine_core::presentation::ui::widget::context_menu::ContextMenu);

            Ok(None)
        } else {
            Ok(None)
        }
    }

    pub fn remove_widget(&self, widget_id: u64) -> bool {
        let registry_binding = WIDGET_REGISTRY.lock();
        registry_binding.borrow_mut().remove(&widget_id).is_some()
    }

    pub fn add_child(&self, parent_id: u64, child_id: u64) -> bool {
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(orig_child) = registry.get_mut(&child_id) {
            orig_child.set_parent(Some(parent_id));
        } else {
            return false;
        }
        let child_clone = registry.get(&child_id).map(|child| child.boxed_clone());
        if let (Some(parent), Some(child)) = (registry.get_mut(&parent_id), child_clone) {
            parent.add_child(child);
            true
        } else {
            false
        }
    }

    pub fn get_children(&self, widget_id: u64) -> Vec<u64> {
        let registry_binding = WIDGET_REGISTRY.lock();
        let registry = registry_binding.borrow();
        if let Some(widget) = registry.get(&widget_id) {
            widget.get_children()
        } else {
            Vec::new()
        }
    }

    pub fn remove_child(&self, parent_id: u64, child_id: u64) -> bool {
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        let mut removed_child: Option<Box<dyn UiWidget + Send>> = None;

        if let Some(parent) = registry.get_mut(&parent_id) {
            // Try Panel
            if let Some(panel) = parent
                .as_any_mut()
                .downcast_mut::<engine_core::presentation::ui::widget::panel::Panel>(
            ) {
                if let Some(pos) = panel.children.iter().position(|c| c.id() == child_id) {
                    let mut child = panel.children.remove(pos);
                    child.set_parent(None);
                    removed_child = Some(child);
                }
            }
            // Try GridLayout (if not already removed)
            if removed_child.is_none() {
                if let Some(grid) = parent
                    .as_any_mut()
                    .downcast_mut::<engine_core::presentation::ui::layout::grid::GridLayout>(
                ) {
                    if let Some(pos) = grid.children.iter().position(|c| c.id() == child_id) {
                        let mut child = grid.children.remove(pos);
                        child.set_parent(None);
                        removed_child = Some(child);
                    }
                }
            }
        }
        if let Some(child) = removed_child {
            registry.insert(child.id(), child);
            true
        } else {
            false
        }
    }

    pub fn set_callback(
        &self,
        widget_id: u64,
        event_name: String,
        callback: PyObject,
    ) -> PyResult<bool> {
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(widget) = registry.get_mut(&widget_id) {
            let cb = Arc::new(move |w: &mut dyn UiWidget| {
                Python::with_gil(|py| {
                    if let Err(e) = callback.call1(py, (w.id(),)) {
                        e.print(py);
                    }
                });
            });
            widget.set_callback(&event_name, Some(cb));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn remove_callback(&self, widget_id: u64, event_name: String) -> PyResult<bool> {
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(widget) = registry.get_mut(&widget_id) {
            widget.set_callback(&event_name, None);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn focus_widget(&self, widget_id: u64) -> bool {
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(widget) = registry.get_mut(&widget_id) {
            widget.set_focused(true);
            true
        } else {
            false
        }
    }

    pub fn trigger_event(
        &self,
        widget_id: u64,
        event_name: String,
        args: Bound<'_, PyAny>,
    ) -> PyResult<bool> {
        use engine_core::presentation::ui::UiEvent;
        let event = match event_name.as_str() {
            "click" => {
                let dict: Bound<'_, PyDict> = args.downcast_into::<PyDict>()?;
                let x = match dict.get_item("x")? {
                    Some(val) => val.extract::<i32>().unwrap_or(0),
                    None => 0,
                };
                let y = match dict.get_item("y")? {
                    Some(val) => val.extract::<i32>().unwrap_or(0),
                    None => 0,
                };
                UiEvent::Click { x, y }
            }
            "key_press" => {
                let dict: Bound<'_, PyDict> = args.downcast_into::<PyDict>()?;
                let key = match dict.get_item("key")? {
                    Some(val) => val.extract::<String>().unwrap_or_default(),
                    None => String::new(),
                };
                UiEvent::KeyPress { key }
            }
            _ => return Ok(false),
        };
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(widget) = registry.get_mut(&widget_id) {
            widget.handle_event(&event);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_widget_type(&self, widget_id: u64) -> Option<String> {
        let registry_binding = WIDGET_REGISTRY.lock();
        let registry = registry_binding.borrow();
        registry
            .get(&widget_id)
            .map(|w| w.widget_type().to_string())
    }

    pub fn get_parent(&self, widget_id: u64) -> Option<u64> {
        let registry_binding = WIDGET_REGISTRY.lock();
        let registry = registry_binding.borrow();
        registry.get(&widget_id).and_then(|w| w.get_parent())
    }

    pub fn set_z_order(&self, widget_id: u64, z: i32) -> bool {
        let registry_binding = WIDGET_REGISTRY.lock();
        let mut registry = registry_binding.borrow_mut();
        if let Some(widget) = registry.get_mut(&widget_id) {
            widget.set_z_order(z);
            true
        } else {
            false
        }
    }

    pub fn get_z_order(&self, widget_id: u64) -> i32 {
        let registry_binding = WIDGET_REGISTRY.lock();
        let registry = registry_binding.borrow();
        registry
            .get(&widget_id)
            .map(|w| w.get_z_order())
            .unwrap_or(0)
    }

    pub fn register_widget(&self, type_name: String, py_ctor: PyObject) -> PyResult<bool> {
        use engine_core::presentation::ui::factory::WidgetProps;
        use engine_core::presentation::ui::widget::dynamic::DynamicWidget;
        use pyo3::Python;

        {
            let factory_binding = UI_FACTORY.lock();
            let factory = factory_binding.borrow();
            if factory.has_widget_type(&type_name) {
                return Ok(false);
            }
        }

        let type_name_for_ctor = type_name.clone();

        let ctor = move |props: WidgetProps| {
            let id = Python::with_gil(|py| {
                let py_props = pyo3::types::PyDict::new(py);
                for (k, v) in &props {
                    let py_val = serde_pyobject::to_pyobject(py, v).unwrap();
                    py_props.set_item(k, py_val).unwrap();
                }
                let res = py_ctor.call1(py, (py_props,));
                match res {
                    Ok(obj) => obj.extract::<u64>(py).unwrap(),
                    Err(e) => {
                        e.print(py);
                        panic!("Python error in custom widget ctor");
                    }
                }
            });
            let registry_binding = WIDGET_REGISTRY.lock();
            let registry = registry_binding.borrow();
            if let Some(w) = registry.get(&id) {
                Box::new(DynamicWidget::new(
                    type_name_for_ctor.clone(),
                    w.boxed_clone(),
                )) as Box<dyn UiWidget + Send>
            } else {
                panic!("Widget with id {id} not found after custom ctor");
            }
        };

        let factory_binding = UI_FACTORY.lock();
        factory_binding
            .borrow_mut()
            .register_widget(&type_name, Box::new(ctor));
        Ok(true)
    }
}

impl Default for UiApi {
    fn default() -> Self {
        Self::new()
    }
}
