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
    #[must_use]
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
        }
    }

    pub fn push(&mut self, event: TickerEvent) {
        if self.max_events == 0 {
            return;
        }
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    #[must_use]
    pub fn recent(&self, count: usize) -> &[TickerEvent] {
        let start = self.events.len().saturating_sub(count);
        self.events.get(start..).unwrap_or(&[])
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str) -> TickerEvent {
        TickerEvent {
            event_kind: kind.to_string(),
            run_id: None,
            shard_id: None,
            step_id: None,
            timestamp: Instant::now(),
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn ticker_new_is_empty() {
        let ticker = EventTicker::new(10);
        assert!(ticker.recent(10).is_empty());
    }

    #[test]
    fn ticker_push_and_recent() {
        let mut ticker = EventTicker::new(10);
        ticker.push(event("A"));
        ticker.push(event("B"));
        ticker.push(event("C"));
        let recent = ticker.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].event_kind, "B");
        assert_eq!(recent[1].event_kind, "C");
    }

    #[test]
    fn ticker_recent_returns_all_when_count_exceeds_len() {
        let mut ticker = EventTicker::new(10);
        ticker.push(event("A"));
        let recent = ticker.recent(100);
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn ticker_evicts_oldest_when_full() {
        let mut ticker = EventTicker::new(2);
        ticker.push(event("first"));
        ticker.push(event("second"));
        ticker.push(event("third"));
        assert_eq!(ticker.recent(10).len(), 2);
        assert_eq!(ticker.recent(10)[0].event_kind, "second");
        assert_eq!(ticker.recent(10)[1].event_kind, "third");
    }

    #[test]
    fn ticker_clear_empties_all() {
        let mut ticker = EventTicker::new(10);
        ticker.push(event("A"));
        ticker.push(event("B"));
        ticker.clear();
        assert!(ticker.recent(10).is_empty());
    }

    #[test]
    fn ticker_zero_capacity_evicts_immediately() {
        let mut ticker = EventTicker::new(0);
        ticker.push(event("gone"));
        assert!(ticker.recent(10).is_empty());
    }

    #[test]
    fn ticker_event_fields_preserved() {
        let evt = TickerEvent {
            event_kind: "StepCompleted".to_string(),
            run_id: Some(99),
            shard_id: Some(3),
            step_id: Some(7),
            timestamp: Instant::now(),
            color: [0.0, 0.961, 1.0, 1.0],
        };
        let mut ticker = EventTicker::new(10);
        ticker.push(evt);
        let recent = ticker.recent(1);
        assert_eq!(recent[0].event_kind, "StepCompleted");
        assert_eq!(recent[0].run_id, Some(99));
        assert_eq!(recent[0].shard_id, Some(3));
        assert_eq!(recent[0].step_id, Some(7));
        assert_eq!(recent[0].color, [0.0, 0.961, 1.0, 1.0]);
    }
}
