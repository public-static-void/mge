use parking_lot::ReentrantMutex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::presentation::ui::widget::{UiWidget, WidgetId};
use once_cell::sync::Lazy;
use serde_json::Value;

pub type WidgetProps = HashMap<String, Value>;

pub type WidgetConstructor = Box<dyn Fn(WidgetProps) -> Box<dyn UiWidget + Send> + Send + Sync>;
pub type WidgetRegistry = HashMap<WidgetId, Box<dyn UiWidget + Send>>;

#[derive(Default)]
pub struct UiFactory {
    registry: HashMap<String, WidgetConstructor>,
}

impl UiFactory {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    pub fn register_widget(&mut self, type_name: &str, constructor: WidgetConstructor) {
        self.registry.insert(type_name.to_string(), constructor);
    }

    pub fn create_widget(
        &self,
        type_name: &str,
        props: WidgetProps,
    ) -> Option<Box<dyn UiWidget + Send>> {
        self.registry.get(type_name).map(|ctor| (ctor)(props))
    }

    pub fn has_widget_type(&self, name: &str) -> bool {
        self.registry.contains_key(name)
    }
}

// Use once_cell for a global singleton
pub static UI_FACTORY: Lazy<Arc<ReentrantMutex<RefCell<UiFactory>>>> =
    Lazy::new(|| Arc::new(ReentrantMutex::new(RefCell::new(UiFactory::new()))));

pub static WIDGET_REGISTRY: Lazy<Arc<ReentrantMutex<RefCell<WidgetRegistry>>>> =
    Lazy::new(|| Arc::new(ReentrantMutex::new(RefCell::new(WidgetRegistry::new()))));
