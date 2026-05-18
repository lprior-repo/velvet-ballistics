#![forbid(unsafe_code)]
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct TickerEvent {
    pub seq: u64,
    pub shard: u32,
    pub run_id: Option<u64>,
    pub kind: TickerEventKind,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TickerEventKind {
    RunAccepted,
    StepStarted,
    StepSucceeded,
    ActionScheduled,
    ActionCompleted,
    ActionFailed,
    RunFinished,
    RunFailed,
    Other,
}

pub struct EventTicker {
    events: VecDeque<TickerEvent>,
    capacity: usize,
    filters: TickerFilters,
}

pub struct TickerFilters {
    pub shard: Option<u32>,
    pub run_id: Option<u64>,
    pub kinds: HashSet<TickerEventKind>,
}

impl EventTicker {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::new(),
            capacity,
            filters: TickerFilters {
                shard: None,
                run_id: None,
                kinds: HashSet::new(),
            },
        }
    }

    pub fn push(&mut self, event: TickerEvent) {
        if self.capacity == 0 {
            return;
        }
        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    #[must_use]
    pub fn events(&self) -> &VecDeque<TickerEvent> {
        &self.events
    }

    #[must_use]
    pub fn filtered_events(&self) -> Vec<&TickerEvent> {
        self.events
            .iter()
            .filter(|event| {
                if let Some(shard) = self.filters.shard
                    && event.shard != shard
                {
                    return false;
                }
                if let Some(run_id) = self.filters.run_id
                    && event.run_id != Some(run_id)
                {
                    return false;
                }
                if !self.filters.kinds.is_empty() && !self.filters.kinds.contains(&event.kind) {
                    return false;
                }
                true
            })
            .collect()
    }

    pub fn set_shard_filter(&mut self, shard: Option<u32>) {
        self.filters.shard = shard;
    }

    pub fn set_run_filter(&mut self, run_id: Option<u64>) {
        self.filters.run_id = run_id;
    }

    pub fn set_kind_filter(&mut self, kinds: HashSet<TickerEventKind>) {
        self.filters.kinds = kinds;
    }

    pub fn clear_filters(&mut self) {
        self.filters.shard = None;
        self.filters.run_id = None;
        self.filters.kinds.clear();
    }

    #[must_use]
    pub fn event_color(kind: TickerEventKind) -> [f32; 4] {
        match kind {
            TickerEventKind::RunAccepted => [0.0, 0.961, 1.0, 1.0],
            TickerEventKind::StepStarted => [0.565, 0.910, 0.071, 1.0],
            TickerEventKind::StepSucceeded => [0.0, 1.0, 0.533, 1.0],
            TickerEventKind::ActionScheduled => [0.475, 0.510, 1.0, 1.0],
            TickerEventKind::ActionCompleted => [0.275, 0.941, 0.941, 1.0],
            TickerEventKind::ActionFailed => [1.0, 0.275, 0.0, 1.0],
            TickerEventKind::RunFinished => [0.565, 1.0, 0.565, 1.0],
            TickerEventKind::RunFailed => [1.0, 0.027, 0.227, 1.0],
            TickerEventKind::Other => [0.6, 0.6, 0.6, 1.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(seq: u64, shard: u32, kind: TickerEventKind) -> TickerEvent {
        TickerEvent {
            seq,
            shard,
            run_id: if seq % 2 == 0 { Some(seq * 10) } else { None },
            kind,
            summary: format!("event-{seq}"),
        }
    }

    fn make_event_with_run(
        seq: u64,
        shard: u32,
        run_id: Option<u64>,
        kind: TickerEventKind,
    ) -> TickerEvent {
        TickerEvent {
            seq,
            shard,
            run_id,
            kind,
            summary: format!("event-{seq}"),
        }
    }

    #[test]
    fn new_ticker_is_empty() {
        let ticker = EventTicker::new(10);
        assert!(ticker.events().is_empty());
        assert_eq!(ticker.events().len(), 0);
    }

    #[test]
    fn push_adds_event_to_buffer() {
        let mut ticker = EventTicker::new(10);
        let evt = make_event(1, 0, TickerEventKind::RunAccepted);
        ticker.push(evt);
        assert_eq!(ticker.events().len(), 1);
        assert_eq!(ticker.events()[0].seq, 1);
        assert_eq!(ticker.events()[0].shard, 0);
        assert_eq!(ticker.events()[0].kind, TickerEventKind::RunAccepted);
    }

    #[test]
    fn push_drops_oldest_at_capacity() {
        let mut ticker = EventTicker::new(3);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 0, TickerEventKind::StepSucceeded));
        ticker.push(make_event(4, 0, TickerEventKind::ActionScheduled));

        assert_eq!(ticker.events().len(), 3);
        assert_eq!(ticker.events()[0].seq, 2);
        assert_eq!(ticker.events()[1].seq, 3);
        assert_eq!(ticker.events()[2].seq, 4);
    }

    #[test]
    fn zero_capacity_rejects_all_events() {
        let mut ticker = EventTicker::new(0);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::RunFailed));
        assert!(ticker.events().is_empty());
    }

    #[test]
    fn filtered_events_returns_all_when_no_filters() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 1, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 2, TickerEventKind::RunFinished));

        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn filtered_events_by_shard() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 1, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 0, TickerEventKind::StepSucceeded));
        ticker.push(make_event(4, 2, TickerEventKind::RunFinished));

        ticker.set_shard_filter(Some(0));
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.shard == 0));
    }

    #[test]
    fn filtered_events_by_run_id() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event_with_run(
            1,
            0,
            Some(100),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            2,
            0,
            Some(200),
            TickerEventKind::StepStarted,
        ));
        ticker.push(make_event_with_run(
            3,
            0,
            Some(100),
            TickerEventKind::StepSucceeded,
        ));
        ticker.push(make_event_with_run(4, 0, None, TickerEventKind::Other));

        ticker.set_run_filter(Some(100));
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.run_id == Some(100)));
    }

    #[test]
    fn filtered_events_by_kind() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::RunFailed));
        ticker.push(make_event(3, 0, TickerEventKind::ActionFailed));
        ticker.push(make_event(4, 0, TickerEventKind::StepSucceeded));

        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunFailed);
        kinds.insert(TickerEventKind::ActionFailed);
        ticker.set_kind_filter(kinds);

        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(
            |e| e.kind == TickerEventKind::RunFailed || e.kind == TickerEventKind::ActionFailed
        ));
    }

    #[test]
    fn filtered_events_with_combined_shard_and_kind() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 1, TickerEventKind::RunFailed));
        ticker.push(make_event(3, 0, TickerEventKind::RunFailed));
        ticker.push(make_event(4, 0, TickerEventKind::StepStarted));

        ticker.set_shard_filter(Some(0));
        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunFailed);
        ticker.set_kind_filter(kinds);

        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].seq, 3);
        assert_eq!(filtered[0].shard, 0);
        assert_eq!(filtered[0].kind, TickerEventKind::RunFailed);
    }

    #[test]
    fn clear_filters_resets_all() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event_with_run(
            1,
            0,
            Some(999),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            2,
            1,
            Some(100),
            TickerEventKind::RunFailed,
        ));

        ticker.set_shard_filter(Some(0));
        ticker.set_run_filter(Some(999));
        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunAccepted);
        ticker.set_kind_filter(kinds);

        assert_eq!(ticker.filtered_events().len(), 1);

        ticker.clear_filters();
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filtered_events_returns_empty_when_no_match() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::StepStarted));

        ticker.set_shard_filter(Some(99));
        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn event_color_returns_neon_colors_per_kind() {
        let cyan = EventTicker::event_color(TickerEventKind::RunAccepted);
        assert_eq!(cyan[3], 1.0);
        assert!(cyan[1] > 0.9);

        let red = EventTicker::event_color(TickerEventKind::RunFailed);
        assert_eq!(red[0], 1.0);
        assert!(red[1] < 0.1);

        let green = EventTicker::event_color(TickerEventKind::StepSucceeded);
        assert!(green[1] > 0.9);
        assert!(green[2] > 0.4);

        let other = EventTicker::event_color(TickerEventKind::Other);
        assert!(other[0] > 0.0 && other[0] < 1.0);
        assert!(other[1] > 0.0 && other[1] < 1.0);
        assert!(other[2] > 0.0 && other[2] < 1.0);
    }

    #[test]
    fn ring_buffer_maintains_fifo_order() {
        let mut ticker = EventTicker::new(5);
        for i in 1..=8u64 {
            ticker.push(make_event(i, 0, TickerEventKind::StepStarted));
        }
        let events = ticker.events();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].seq, 4);
        assert_eq!(events[4].seq, 8);
    }

    #[test]
    fn run_id_none_events_excluded_by_run_filter() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event_with_run(1, 0, None, TickerEventKind::Other));
        ticker.push(make_event_with_run(
            2,
            0,
            Some(42),
            TickerEventKind::RunAccepted,
        ));

        ticker.set_run_filter(Some(42));
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].seq, 2);
    }

    #[test]
    fn capacity_one_keeps_single_latest_event() {
        let mut ticker = EventTicker::new(1);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::RunFailed));
        assert_eq!(ticker.events().len(), 1);
        assert_eq!(ticker.events()[0].seq, 2);
        assert_eq!(ticker.events()[0].kind, TickerEventKind::RunFailed);
    }

    // -------------------------------------------------------------------------
    // Additional tests for broader coverage
    // -------------------------------------------------------------------------

    #[test]
    fn filter_by_shard_excludes_non_matching() {
        let mut ticker = EventTicker::new(20);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 1, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 2, TickerEventKind::StepSucceeded));
        ticker.push(make_event(4, 1, TickerEventKind::ActionScheduled));
        ticker.push(make_event(5, 0, TickerEventKind::RunFinished));

        ticker.set_shard_filter(Some(1));
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 2);
        let seqs: Vec<u64> = filtered.iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![2, 4]);
    }

    #[test]
    fn filter_by_run_id_with_multiple_run_ids() {
        let mut ticker = EventTicker::new(20);
        ticker.push(make_event_with_run(
            1,
            0,
            Some(10),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            2,
            0,
            Some(20),
            TickerEventKind::StepStarted,
        ));
        ticker.push(make_event_with_run(
            3,
            0,
            Some(10),
            TickerEventKind::StepSucceeded,
        ));
        ticker.push(make_event_with_run(
            4,
            0,
            Some(30),
            TickerEventKind::ActionCompleted,
        ));
        ticker.push(make_event_with_run(
            5,
            0,
            Some(10),
            TickerEventKind::RunFinished,
        ));

        ticker.set_run_filter(Some(10));
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().all(|e| e.run_id == Some(10)));
    }

    #[test]
    fn filter_by_kind_single_kind() {
        let mut ticker = EventTicker::new(20);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(4, 0, TickerEventKind::ActionFailed));

        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunAccepted);
        ticker.set_kind_filter(kinds);

        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 2);
        assert!(
            filtered
                .iter()
                .all(|e| e.kind == TickerEventKind::RunAccepted)
        );
    }

    #[test]
    fn ring_buffer_eviction_at_exact_capacity_boundary() {
        // Push exactly `capacity` events, verify nothing evicted.
        let mut ticker = EventTicker::new(5);
        for i in 1..=5u64 {
            ticker.push(make_event(i, 0, TickerEventKind::StepStarted));
        }
        assert_eq!(ticker.events().len(), 5);
        assert_eq!(ticker.events()[0].seq, 1);
        assert_eq!(ticker.events()[4].seq, 5);

        // Push one more to trigger eviction of the oldest.
        ticker.push(make_event(6, 0, TickerEventKind::StepStarted));
        assert_eq!(ticker.events().len(), 5);
        assert_eq!(ticker.events()[0].seq, 2);
        assert_eq!(ticker.events()[4].seq, 6);
    }

    #[test]
    fn click_to_jump_finds_event_by_seq() {
        // Simulates click-to-jump: find an event by seq within the buffer.
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(10, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(20, 1, TickerEventKind::StepSucceeded));
        ticker.push(make_event(30, 0, TickerEventKind::ActionScheduled));

        let target = ticker.events().iter().find(|e| e.seq == 20);
        let Some(found) = target else { return };
        assert_eq!(found.seq, 20);
        assert_eq!(found.shard, 1);
        assert_eq!(found.kind, TickerEventKind::StepSucceeded);
    }

    #[test]
    fn click_to_jump_returns_none_for_evicted_event() {
        let mut ticker = EventTicker::new(3);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 0, TickerEventKind::StepSucceeded));
        ticker.push(make_event(4, 0, TickerEventKind::ActionScheduled));

        // Event with seq=1 has been evicted.
        let found = ticker.events().iter().find(|e| e.seq == 1);
        assert!(found.is_none());
    }

    #[test]
    fn empty_ticker_filtered_events_is_empty() {
        let ticker = EventTicker::new(10);
        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn empty_ticker_with_active_filters_is_empty() {
        let mut ticker = EventTicker::new(10);
        ticker.set_shard_filter(Some(0));
        ticker.set_run_filter(Some(1));
        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunAccepted);
        ticker.set_kind_filter(kinds);

        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn all_kind_variants_have_valid_colors() {
        let all_kinds = [
            TickerEventKind::RunAccepted,
            TickerEventKind::StepStarted,
            TickerEventKind::StepSucceeded,
            TickerEventKind::ActionScheduled,
            TickerEventKind::ActionCompleted,
            TickerEventKind::ActionFailed,
            TickerEventKind::RunFinished,
            TickerEventKind::RunFailed,
            TickerEventKind::Other,
        ];
        for kind in all_kinds {
            let color = EventTicker::event_color(kind);
            // Alpha channel must be fully opaque (1.0) for all kinds.
            let Some(&alpha) = color.get(3) else { return };
            assert!(
                (alpha - 1.0f32).abs() < f32::EPSILON,
                "alpha for {kind:?} must be 1.0"
            );
            // All RGBA components must be in [0.0, 1.0].
            for (i, &c) in color.iter().enumerate() {
                assert!(
                    c >= 0.0 && c <= 1.0,
                    "component {i} for {kind:?} out of range: {c}"
                );
            }
        }
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: Medium
    /// DESCRIPTION: EventTicker::push uses VecDeque::pop_front() for eviction
    /// which is O(1), but VecDeque::push_back may reallocate if the deque
    /// exceeds its capacity. With capacity set very large, a rapid burst of
    /// events could cause a large allocation spike. More importantly, if
    /// capacity is set to usize::MAX, the push never evicts and memory grows
    /// without bound. There is no upper bound on the capacity parameter.
    #[test]
    fn blackhat_huge_capacity_allows_unbounded_growth() {
        let mut ticker = EventTicker::new(usize::MAX);
        // Push many events -- none are evicted.
        for i in 0..1000u64 {
            ticker.push(make_event(i, 0, TickerEventKind::StepStarted));
        }
        assert_eq!(ticker.events().len(), 1000);
        // This demonstrates that a misconfigured capacity allows unbounded
        // memory growth. The API has no guard against this.
    }

    /// SEVERITY: Low
    /// DESCRIPTION: TickerFilters stores a HashSet<TickerEventKind> which is
    /// heap-allocated. Each set_kind_filter call replaces the entire set.
    /// If called frequently with large sets, this creates allocation pressure.
    /// Additionally, the kinds HashSet grows on each insert and is never
    /// shrunk, potentially holding excess capacity.
    #[test]
    fn blackhat_kind_filter_allocation_pattern() {
        let mut ticker = EventTicker::new(10);
        // Set and reset filters repeatedly.
        for _ in 0..100 {
            let mut kinds = HashSet::new();
            kinds.insert(TickerEventKind::RunFailed);
            kinds.insert(TickerEventKind::ActionFailed);
            ticker.set_kind_filter(kinds);
            ticker.clear_filters();
        }
        // Verify filters are cleared after the loop.
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        assert_eq!(ticker.filtered_events().len(), 1);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: TickerEvent::seq is u64 but is never validated or used
    /// for ordering. Events with non-monotonic seq values (e.g., seq going
    /// backwards) are accepted without warning. The seq field is purely
    /// informational and carries no ordering guarantee within the buffer.
    #[test]
    fn blackhat_non_monotonic_seq_accepted_without_warning() {
        let mut ticker = EventTicker::new(10);
        ticker.push(TickerEvent {
            seq: 100,
            shard: 0,
            run_id: None,
            kind: TickerEventKind::RunAccepted,
            summary: "future".to_string(),
        });
        ticker.push(TickerEvent {
            seq: 50,
            shard: 0,
            run_id: None,
            kind: TickerEventKind::StepStarted,
            summary: "past".to_string(),
        });
        // Both accepted; no ordering enforcement.
        assert_eq!(ticker.events().len(), 2);
        assert_eq!(ticker.events()[0].seq, 100);
        assert_eq!(ticker.events()[1].seq, 50);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: filtered_events() allocates a Vec of references every call.
    /// For high-frequency polling (e.g., per-frame at 60fps), this creates
    /// continuous allocation pressure. The returned Vec is short-lived but
    /// causes heap churn.
    #[test]
    fn blackhat_filtered_events_allocates_each_call() {
        let mut ticker = EventTicker::new(100);
        for i in 0..50 {
            ticker.push(make_event(
                i,
                u32::try_from(i % 3).unwrap_or(u32::MAX),
                TickerEventKind::StepStarted,
            ));
        }
        // Call filtered_events multiple times -- each allocates a new Vec.
        let f1 = ticker.filtered_events();
        let f2 = ticker.filtered_events();
        assert_eq!(f1.len(), f2.len());
    }

    // =========================================================================
    // Additional comprehensive coverage tests
    // =========================================================================

    #[test]
    fn ticker_event_clone_preserves_fields() {
        let evt = TickerEvent {
            seq: 42,
            shard: 3,
            run_id: Some(999),
            kind: TickerEventKind::ActionCompleted,
            summary: "test event".to_string(),
        };
        let cloned = evt.clone();
        assert_eq!(cloned.seq, 42);
        assert_eq!(cloned.shard, 3);
        assert_eq!(cloned.run_id, Some(999));
        assert_eq!(cloned.kind, TickerEventKind::ActionCompleted);
        assert_eq!(cloned.summary, "test event");
    }

    #[test]
    fn ticker_event_debug_format() {
        let evt = TickerEvent {
            seq: 1,
            shard: 0,
            run_id: None,
            kind: TickerEventKind::RunAccepted,
            summary: "hello".to_string(),
        };
        let debug = format!("{evt:?}");
        assert!(debug.contains("seq"));
        assert!(debug.contains("shard"));
    }

    #[test]
    fn ticker_event_kind_equality() {
        assert_eq!(TickerEventKind::RunAccepted, TickerEventKind::RunAccepted);
        assert_ne!(TickerEventKind::RunAccepted, TickerEventKind::RunFailed);
    }

    #[test]
    fn ticker_event_kind_copy() {
        let kind = TickerEventKind::StepStarted;
        let copied = kind;
        assert_eq!(kind, copied);
    }

    #[test]
    fn ticker_event_kind_hash_consistency() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TickerEventKind::RunAccepted);
        set.insert(TickerEventKind::RunAccepted);
        assert_eq!(set.len(), 1);
        set.insert(TickerEventKind::RunFailed);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn push_events_maintain_insertion_order() {
        let mut ticker = EventTicker::new(10);
        for i in 0..5u64 {
            ticker.push(TickerEvent {
                seq: i,
                shard: 0,
                run_id: None,
                kind: TickerEventKind::StepStarted,
                summary: format!("step-{i}"),
            });
        }
        let events = ticker.events();
        for (idx, evt) in events.iter().enumerate() {
            assert_eq!(
                evt.seq,
                u64::try_from(idx).unwrap_or(u64::MAX),
                "events must be in insertion order"
            );
        }
    }

    #[test]
    fn set_shard_filter_none_removes_filter() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 1, TickerEventKind::StepStarted));

        ticker.set_shard_filter(Some(0));
        assert_eq!(ticker.filtered_events().len(), 1);

        ticker.set_shard_filter(None);
        assert_eq!(ticker.filtered_events().len(), 2);
    }

    #[test]
    fn set_run_filter_none_removes_filter() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event_with_run(
            1,
            0,
            Some(100),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            2,
            0,
            Some(200),
            TickerEventKind::StepStarted,
        ));

        ticker.set_run_filter(Some(100));
        assert_eq!(ticker.filtered_events().len(), 1);

        ticker.set_run_filter(None);
        assert_eq!(ticker.filtered_events().len(), 2);
    }

    #[test]
    fn set_kind_filter_empty_set_removes_filter() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 0, TickerEventKind::StepStarted));

        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunAccepted);
        ticker.set_kind_filter(kinds);
        assert_eq!(ticker.filtered_events().len(), 1);

        ticker.set_kind_filter(HashSet::new());
        assert_eq!(ticker.filtered_events().len(), 2);
    }

    #[test]
    fn combined_filters_all_three_simultaneously() {
        let mut ticker = EventTicker::new(20);
        ticker.push(make_event_with_run(
            1,
            0,
            Some(10),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            2,
            0,
            Some(10),
            TickerEventKind::StepStarted,
        ));
        ticker.push(make_event_with_run(
            3,
            1,
            Some(10),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            4,
            0,
            Some(20),
            TickerEventKind::RunAccepted,
        ));
        ticker.push(make_event_with_run(
            5,
            0,
            Some(10),
            TickerEventKind::ActionFailed,
        ));

        // Shard 0, run 10, kind RunAccepted -> only event 1
        ticker.set_shard_filter(Some(0));
        ticker.set_run_filter(Some(10));
        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunAccepted);
        ticker.set_kind_filter(kinds);

        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].seq, 1);
    }

    #[test]
    fn event_color_all_kinds_have_nonzero_alpha() {
        let all_kinds = [
            TickerEventKind::RunAccepted,
            TickerEventKind::StepStarted,
            TickerEventKind::StepSucceeded,
            TickerEventKind::ActionScheduled,
            TickerEventKind::ActionCompleted,
            TickerEventKind::ActionFailed,
            TickerEventKind::RunFinished,
            TickerEventKind::RunFailed,
            TickerEventKind::Other,
        ];
        for kind in all_kinds {
            let color = EventTicker::event_color(kind);
            assert!(color[3] > 0.0, "alpha for {kind:?} must be nonzero");
        }
    }

    #[test]
    fn event_color_action_completed_is_teal() {
        let color = EventTicker::event_color(TickerEventKind::ActionCompleted);
        assert!(color[0] > 0.0 && color[0] < 0.5);
        assert!(color[1] > 0.9);
        assert!(color[2] > 0.9);
    }

    #[test]
    fn event_color_action_scheduled_is_blue() {
        let color = EventTicker::event_color(TickerEventKind::ActionScheduled);
        assert!(color[0] > 0.3 && color[0] < 0.6);
        assert!(color[1] > 0.4 && color[1] < 0.6);
        assert!((color[2] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn event_color_run_finished_is_light_green() {
        let color = EventTicker::event_color(TickerEventKind::RunFinished);
        assert!(color[0] > 0.4);
        assert!((color[1] - 1.0).abs() < f32::EPSILON);
        assert!(color[2] > 0.4);
    }

    #[test]
    fn new_ticker_has_no_filters_active() {
        let ticker = EventTicker::new(10);
        // filtered_events with no filters returns all events (which is none).
        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn filter_events_does_not_mutate_underlying_buffer() {
        let mut ticker = EventTicker::new(10);
        ticker.push(make_event(1, 0, TickerEventKind::RunAccepted));
        ticker.push(make_event(2, 1, TickerEventKind::StepStarted));
        ticker.push(make_event(3, 0, TickerEventKind::StepSucceeded));

        ticker.set_shard_filter(Some(0));
        let _ = ticker.filtered_events();
        // Original buffer should be unchanged.
        assert_eq!(ticker.events().len(), 3);
    }

    #[test]
    fn repeated_push_and_filter_cycle() {
        let mut ticker = EventTicker::new(5);
        for round in 0u64..3 {
            ticker.push(make_event(round, 0, TickerEventKind::RunAccepted));
        }
        ticker.set_shard_filter(Some(0));
        let filtered = ticker.filtered_events();
        assert_eq!(filtered.len(), 3);

        ticker.clear_filters();
        ticker.set_shard_filter(Some(99));
        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn ticker_with_large_capacity_accepts_many_events() {
        let mut ticker = EventTicker::new(10_000);
        for i in 0..500u64 {
            ticker.push(make_event(i, 0, TickerEventKind::StepStarted));
        }
        assert_eq!(ticker.events().len(), 500);
    }

    #[test]
    fn filter_shard_with_no_matching_events() {
        let mut ticker = EventTicker::new(10);
        // All events are on shard 0.
        for i in 0..5u64 {
            ticker.push(make_event(i, 0, TickerEventKind::StepStarted));
        }
        // Filter for shard 99 -> no matches.
        ticker.set_shard_filter(Some(99));
        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn filter_kind_with_no_matching_events() {
        let mut ticker = EventTicker::new(10);
        for i in 0..5u64 {
            ticker.push(make_event(i, 0, TickerEventKind::RunAccepted));
        }
        let mut kinds = HashSet::new();
        kinds.insert(TickerEventKind::RunFailed);
        ticker.set_kind_filter(kinds);
        assert!(ticker.filtered_events().is_empty());
    }

    #[test]
    fn make_event_helper_produces_correct_run_id_pattern() {
        // Even seq -> Some(seq * 10), odd seq -> None
        let even_evt = make_event(4, 0, TickerEventKind::RunAccepted);
        assert_eq!(even_evt.run_id, Some(40));
        let odd_evt = make_event(3, 0, TickerEventKind::RunAccepted);
        assert_eq!(odd_evt.run_id, None);
    }

    #[test]
    fn ticker_event_summary_field_preserved() {
        let mut ticker = EventTicker::new(10);
        ticker.push(TickerEvent {
            seq: 1,
            shard: 0,
            run_id: None,
            kind: TickerEventKind::Other,
            summary: "custom summary text".to_string(),
        });
        assert_eq!(ticker.events()[0].summary, "custom summary text");
    }
}
