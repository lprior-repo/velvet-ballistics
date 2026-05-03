use std::time::Instant;

pub struct EventTicker {
    events: Vec<TickerEvent>,
    max_events: usize,
}

#[derive(Debug, Clone)]
pub struct TickerEvent {
    pub event_kind: String,
    pub run_id: Option<u64>,
    pub shard_id: Option<u32>,
    pub step_id: Option<u16>,
    pub timestamp: Instant,
    pub color: [f32; 4],
}

impl EventTicker {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
        }
    }

    pub fn push(&mut self, event: TickerEvent) {
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    pub fn recent(&self, count: usize) -> &[TickerEvent] {
        let start = self.events.len().saturating_sub(count);
        &self.events[start..]
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
