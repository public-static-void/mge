use serde_json::Value as JsonValue;
use std::collections::VecDeque;

#[derive(Default)]
pub struct AiEventIntentQueue {
    pub intents: VecDeque<JsonValue>,
}
