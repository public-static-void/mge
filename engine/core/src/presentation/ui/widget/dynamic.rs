use crate::presentation::renderer::PresentationRenderer;
use crate::presentation::ui::UiEvent;
use crate::presentation::ui::widget::{UiWidget, WidgetId};
use std::any::Any;
use std::sync::Arc;

pub struct DynamicWidget {
    type_name: String,
    inner: Box<dyn UiWidget + Send>,
}

impl DynamicWidget {
    pub fn new(type_name: String, inner: Box<dyn UiWidget + Send>) -> Self {
        Self { type_name, inner }
    }
}

impl UiWidget for DynamicWidget {
    fn id(&self) -> WidgetId {
        self.inner.id()
    }
    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        self.inner.render(renderer)
    }
    fn handle_event(&mut self, event: &UiEvent) {
        self.inner.handle_event(event)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn is_focusable(&self) -> bool {
        self.inner.is_focusable()
    }
    fn set_focused(&mut self, focused: bool) {
        self.inner.set_focused(focused)
    }
    fn is_focused(&self) -> bool {
        self.inner.is_focused()
    }
    fn focus_pos(&self) -> (i32, i32) {
        self.inner.focus_pos()
    }
    fn focus_group(&self) -> Option<u32> {
        self.inner.focus_group()
    }
    fn set_callback(
        &mut self,
        event: &str,
        cb: Option<Arc<dyn Fn(&mut dyn UiWidget) + Send + Sync>>,
    ) {
        self.inner.set_callback(event, cb)
    }
    fn add_child(&mut self, child: Box<dyn UiWidget + Send>) {
        self.inner.add_child(child)
    }
    fn get_children(&self) -> Vec<u64> {
        self.inner.get_children()
    }
    fn set_props(&mut self, props: &std::collections::HashMap<String, serde_json::Value>) {
        self.inner.set_props(props)
    }
    fn widget_type(&self) -> &'static str {
        // SAFETY: We leak the string to get a &'static str, which is fine for type names.
        Box::leak(self.type_name.clone().into_boxed_str())
    }
    fn get_parent(&self) -> Option<WidgetId> {
        self.inner.get_parent()
    }
    fn set_parent(&mut self, parent: Option<WidgetId>) {
        self.inner.set_parent(parent)
    }
    fn set_z_order(&mut self, z: i32) {
        self.inner.set_z_order(z)
    }
    fn get_z_order(&self) -> i32 {
        self.inner.get_z_order()
    }
    fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
        Box::new(DynamicWidget {
            type_name: self.type_name.clone(),
            inner: self.inner.boxed_clone(),
        })
    }
}
