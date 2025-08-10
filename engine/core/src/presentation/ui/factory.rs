use parking_lot::ReentrantMutex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::presentation::ui::widget::{UiWidget, WidgetId};
use once_cell::sync::Lazy;
use serde_json::Value;

/// Widget properties
pub type WidgetProps = HashMap<String, Value>;

/// Widget constructor
pub type WidgetConstructor = Box<dyn Fn(WidgetProps) -> Box<dyn UiWidget + Send> + Send + Sync>;

/// Widget registry
pub type WidgetRegistry = HashMap<WidgetId, Box<dyn UiWidget + Send>>;

/// UI factory
#[derive(Default)]
pub struct UiFactory {
    registry: HashMap<String, WidgetConstructor>,
}

impl UiFactory {
    /// Create new factory
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// Register widget
    pub fn register_widget(&mut self, type_name: &str, constructor: WidgetConstructor) {
        self.registry.insert(type_name.to_string(), constructor);
    }

    /// Create widget
    pub fn create_widget(
        &self,
        type_name: &str,
        props: WidgetProps,
    ) -> Option<Box<dyn UiWidget + Send>> {
        self.registry.get(type_name).map(|ctor| (ctor)(props))
    }

    /// Check if widget type is registered
    pub fn has_widget_type(&self, name: &str) -> bool {
        self.registry.contains_key(name)
    }
}

/// Global UI factory
/// Use once_cell for a global singleton
pub static UI_FACTORY: Lazy<Arc<ReentrantMutex<RefCell<UiFactory>>>> =
    Lazy::new(|| Arc::new(ReentrantMutex::new(RefCell::new(UiFactory::new()))));

/// Global widget registry
pub static WIDGET_REGISTRY: Lazy<Arc<ReentrantMutex<RefCell<WidgetRegistry>>>> =
    Lazy::new(|| Arc::new(ReentrantMutex::new(RefCell::new(WidgetRegistry::new()))));
