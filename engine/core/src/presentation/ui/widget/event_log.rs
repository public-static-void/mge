use crate::ecs::event_logger::{EventLogger, LoggedEvent};
use crate::presentation::renderer::{PresentationRenderer, RenderColor, RenderCommand};
use crate::presentation::ui::widget::widget_trait::WidgetCallback;
use crate::presentation::ui::{UiEvent, widget};
use crate::systems::job::system::events::job_event_logger;
use serde_json::Value as JsonValue;
use std::any::Any;
use std::sync::Arc;

pub struct EventLogWidget {
    logger: Arc<EventLogger<JsonValue>>,
    filter: String,
    selected_event: Option<usize>,
    events: Vec<LoggedEvent<JsonValue>>,
    id: widget::WidgetId,
    parent: Option<widget::WidgetId>,
    z_order: i32,
}

impl EventLogWidget {
    pub fn new(id: widget::WidgetId) -> Self {
        Self {
            logger: job_event_logger(),
            filter: String::new(),
            selected_event: None,
            events: Vec::new(),
            id,
            parent: None,
            z_order: 0,
        }
    }

    pub fn update(&mut self) {
        self.events = self.logger.query_events(|_| true);
        if !self.filter.is_empty() {
            self.events.retain(|e| e.event_type.contains(&self.filter));
        }
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
    }
}

impl widget::UiWidget for EventLogWidget {
    fn id(&self) -> widget::WidgetId {
        self.id
    }

    fn render(&mut self, renderer: &mut dyn PresentationRenderer) {
        for (row, event) in self.events.iter().enumerate() {
            let text = format!("{}: {}", event.event_type, event.timestamp);
            for (col, ch) in text.chars().enumerate() {
                renderer.queue_draw(RenderCommand {
                    glyph: ch,
                    color: RenderColor(255, 255, 255),
                    pos: (col as i32, row as i32),
                });
            }
        }
        if let Some(idx) = self.selected_event
            && let Some(event) = self.events.get(idx)
        {
            let details = format!(
                "Type: {}\nTimestamp: {}\nPayload: {}",
                event.event_type,
                event.timestamp,
                serde_json::to_string_pretty(&event.payload).unwrap()
            );
            let start_row = self.events.len() as i32;
            for (i, line) in details.lines().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    renderer.queue_draw(RenderCommand {
                        glyph: ch,
                        color: RenderColor(255, 255, 255),
                        pos: (col as i32, start_row + i as i32),
                    });
                }
            }
        }
    }

    fn handle_event(&mut self, _event: &UiEvent) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn is_focused(&self) -> bool {
        false
    }

    fn focus_pos(&self) -> (i32, i32) {
        (0, 0)
    }

    fn focus_group(&self) -> Option<u32> {
        None
    }

    fn set_callback(&mut self, _event: &str, _cb: Option<WidgetCallback>) {}

    fn add_child(&mut self, _child: Box<dyn widget::UiWidget + Send>) {}

    fn get_children(&self) -> Vec<widget::WidgetId> {
        Vec::new()
    }

    fn set_props(&mut self, _props: &std::collections::HashMap<String, JsonValue>) {}

    fn widget_type(&self) -> &'static str {
        "event_log"
    }

    fn get_parent(&self) -> Option<widget::WidgetId> {
        self.parent
    }

    fn set_parent(&mut self, parent: Option<widget::WidgetId>) {
        self.parent = parent;
    }

    fn set_z_order(&mut self, z: i32) {
        self.z_order = z;
    }

    fn get_z_order(&self) -> i32 {
        self.z_order
    }

    fn boxed_clone(&self) -> Box<dyn widget::UiWidget + Send> {
        let mut clone = Self::new(self.id);
        clone.filter = self.filter.clone();
        clone.selected_event = self.selected_event;
        clone.events = self.events.clone();
        clone.parent = self.parent;
        clone.z_order = self.z_order;
        Box::new(clone)
    }
}
