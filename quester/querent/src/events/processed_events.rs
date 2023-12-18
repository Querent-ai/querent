use querent_synapse::callbacks::{EventState, EventType};
use std::fmt;

pub struct ProcessedEvent {
    pub event_state: EventState,
    pub event_type: EventType,
}

impl fmt::Debug for ProcessedEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessedEvent")
            .field("event_state", &self.event_state)
            .field("event_type", &self.event_type)
            .finish()
    }
}

pub struct ProcessedEventsBatch {
    pub events: Vec<ProcessedEvent>,
    pub checkpoint_delta: Option<i64>,
    pub force_commit: bool,
}

impl fmt::Debug for ProcessedEventsBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessedEventsBatch")
            .field("events", &self.events)
            .field("checkpoint_delta", &self.checkpoint_delta)
            .field("force_commit", &self.force_commit)
            .finish()
    }
}
