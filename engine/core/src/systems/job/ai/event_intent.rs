use serde_json::Value as JsonValue;
use std::collections::VecDeque;

#[derive(Default)]
/// Intent queue
pub struct AiEventIntentQueue {
    /// Intents
    pub intents: VecDeque<JsonValue>,
}
