//! Hot-path / latency overlay for the System Overview screen.
//!
//! Tracks per-segment duration samples in fixed-size ring buffers and exposes
//! min / max / avg / p50 / p99 statistics without unbounded memory growth.
//!
//! Canonical pipeline segments:
//!   submit -> admit:  0.3 ms
//!   admit -> first step: 0.1 ms
//!   first step -> action scheduled: 12 ms
//!   action scheduled -> completed: 3.2 s
//!   completed -> finish: 0.2 ms

/// Capacity of the per-segment ring buffer.  Keeps memory bounded while still
/// providing enough samples for stable percentile estimates.
const RING_CAPACITY: usize = 1024;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Summary statistics for one named pipeline segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatencySegment {
    /// Human-readable label, e.g. `"submit -> admit"`.
    pub label: &'static str,
    /// Minimum observed duration in microseconds.
    pub min_us: u64,
    /// Maximum observed duration in microseconds.
    pub max_us: u64,
    /// Arithmetic mean duration in microseconds.
    pub avg_us: u64,
    /// 50th percentile duration in microseconds.
    pub p50_us: u64,
    /// 99th percentile duration in microseconds.
    pub p99_us: u64,
    /// Total number of samples recorded (including those evicted from the ring).
    pub sample_count: u64,
}

// ---------------------------------------------------------------------------
// Internal ring buffer
// ---------------------------------------------------------------------------

/// Fixed-size ring buffer that stores `u64` microsecond samples.
struct SampleRing {
    buf: Vec<u64>,
    head: usize,
    len: usize,
}

impl SampleRing {
    fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0; capacity],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, value: u64) {
        if let Some(slot) = self.buf.get_mut(self.head) {
            *slot = value;
        }
        self.head = self
            .head
            .saturating_add(1)
            .checked_rem(self.buf.len())
            .unwrap_or(0);
        if self.len < self.buf.len() {
            self.len = self.len.saturating_add(1);
        }
    }

    fn as_sorted_slice<'a>(&self, scratch: &'a mut Vec<u64>) -> &'a [u64] {
        scratch.clear();
        scratch.extend_from_slice(
            self.buf
                .get(..self.len.min(self.buf.len()))
                .unwrap_or(&[]),
        );
        scratch.sort_unstable();
        scratch
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ---------------------------------------------------------------------------
// Internal per-label accumulator
// ---------------------------------------------------------------------------

struct SegmentAccumulator {
    label: &'static str,
    ring: SampleRing,
    total_us: u64,
    total_count: u64,
    min_us: u64,
    max_us: u64,
}

impl SegmentAccumulator {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            ring: SampleRing::new(RING_CAPACITY),
            total_us: 0,
            total_count: 0,
            min_us: u64::MAX,
            max_us: 0,
        }
    }

    fn record(&mut self, duration_us: u64) {
        self.ring.push(duration_us);
        self.total_us = self.total_us.saturating_add(duration_us);
        self.total_count = self.total_count.saturating_add(1);
        if duration_us < self.min_us {
            self.min_us = duration_us;
        }
        if duration_us > self.max_us {
            self.max_us = duration_us;
        }
    }

    fn compute_segment(&self, scratch: &mut Vec<u64>) -> Option<LatencySegment> {
        if self.ring.is_empty() {
            return None;
        }
        let sorted = self.ring.as_sorted_slice(scratch);
        let p50_us = percentile(sorted, 50);
        let p99_us = percentile(sorted, 99);
        let avg_us = self.total_us.checked_div(self.total_count).unwrap_or(0);
        Some(LatencySegment {
            label: self.label,
            min_us: self.min_us,
            max_us: self.max_us,
            avg_us,
            p50_us,
            p99_us,
            sample_count: self.total_count,
        })
    }
}

// ---------------------------------------------------------------------------
// Percentile helper
// ---------------------------------------------------------------------------

/// Nearest-rank percentile on a **pre-sorted** slice.
/// Returns 0 for an empty slice.
fn percentile(sorted: &[u64], pct: u8) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    // Clamp pct to 0..=100 then compute nearest-rank index.
    let pct_clamped = usize::from(pct.min(100));
    // nearest-rank: index = ceil(pct/100 * N) - 1
    let rank = pct_clamped
        .saturating_mul(sorted.len())
        .checked_div(100)
        .unwrap_or(0);
    // rank is in 1..=N, convert to 0-based index clamped to last element.
    let idx = rank.saturating_sub(1).min(sorted.len().saturating_sub(1));
    *sorted.get(idx).unwrap_or(&0)
}

// ---------------------------------------------------------------------------
// LatencyProfile -- the main public API
// ---------------------------------------------------------------------------

/// Aggregates latency samples across named pipeline segments and computes
/// summary statistics on demand.
pub struct LatencyProfile {
    accumulators: Vec<SegmentAccumulator>,
}

impl LatencyProfile {
    /// Create an empty profile with no segments.
    #[must_use]
    pub fn new() -> Self {
        Self {
            accumulators: Vec::new(),
        }
    }

    /// Record a single duration sample for the given `label`.
    ///
    /// If the label has not been seen before a new segment accumulator is
    /// created automatically.
    pub fn record(&mut self, label: &'static str, duration_us: u64) {
        if let Some(acc) = self.accumulators.iter_mut().find(|a| a.label == label) {
            acc.record(duration_us);
        } else {
            let mut acc = SegmentAccumulator::new(label);
            acc.record(duration_us);
            self.accumulators.push(acc);
        }
    }

    /// Return current summary statistics for every segment that has at least
    /// one sample.
    pub fn segments(&self) -> Vec<LatencySegment> {
        // We need &mut for the scratch buffer but the method signature takes
        // &self.  Create a local scratch here so the public API stays clean.
        let mut scratch = Vec::new();
        self.accumulators
            .iter()
            .filter_map(|acc| acc.compute_segment(&mut scratch))
            .collect()
    }

    /// Return the segment with the highest average latency.
    /// Returns `None` when no samples have been recorded.
    pub fn slowest_segment(&self) -> Option<LatencySegment> {
        let segs = self.segments();
        segs.iter().max_by_key(|s| s.avg_us).cloned()
    }

    /// Return the sum of all segment averages in microseconds.
    /// Useful for displaying the end-to-end hot-path latency.
    pub fn total_avg_us(&self) -> u64 {
        self.segments()
            .iter()
            .fold(0u64, |acc, s| acc.saturating_add(s.avg_us))
    }
}

impl Default for LatencyProfile {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- helper ---------------------------------------------------------------
    fn record_n(profile: &mut LatencyProfile, label: &'static str, values: &[u64]) {
        for &v in values {
            profile.record(label, v);
        }
    }

    // -- tests ----------------------------------------------------------------

    #[test]
    fn new_profile_has_no_segments() {
        let p = LatencyProfile::new();
        assert!(p.segments().is_empty());
        assert!(p.slowest_segment().is_none());
        assert_eq!(p.total_avg_us(), 0);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(LatencyProfile::default().segments(), LatencyProfile::new().segments());
    }

    #[test]
    fn single_record_produces_one_segment_with_correct_stats() {
        let mut p = LatencyProfile::new();
        p.record("submit -> admit", 300);
        let segs = p.segments();
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        assert_eq!(s.label, "submit -> admit");
        assert_eq!(s.min_us, 300);
        assert_eq!(s.max_us, 300);
        assert_eq!(s.avg_us, 300);
        assert_eq!(s.p50_us, 300);
        assert_eq!(s.p99_us, 300);
        assert_eq!(s.sample_count, 1);
    }

    #[test]
    fn multiple_records_compute_min_max_avg() {
        let mut p = LatencyProfile::new();
        // values: 100, 200, 300 -> min=100, max=300, avg=200
        record_n(&mut p, "seg", &[100, 200, 300]);
        let segs = p.segments();
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        assert_eq!(s.min_us, 100);
        assert_eq!(s.max_us, 300);
        assert_eq!(s.avg_us, 200);
        assert_eq!(s.sample_count, 3);
    }

    #[test]
    fn percentile_p50_on_sorted_values() {
        let mut p = LatencyProfile::new();
        // 1..=100 -> p50 should be 50
        let vals: Vec<u64> = (1..=100).collect();
        record_n(&mut p, "p50test", &vals);
        let segs = p.segments();
        let s = &segs[0];
        assert_eq!(s.p50_us, 50);
    }

    #[test]
    fn percentile_p99_on_large_sample() {
        let mut p = LatencyProfile::new();
        // 1..=100 -> p99 should be 99
        let vals: Vec<u64> = (1..=100).collect();
        record_n(&mut p, "p99test", &vals);
        let segs = p.segments();
        let s = &segs[0];
        assert_eq!(s.p99_us, 99);
    }

    #[test]
    fn multiple_labels_create_separate_segments() {
        let mut p = LatencyProfile::new();
        p.record("alpha", 10);
        p.record("beta", 20);
        p.record("alpha", 30);
        let segs = p.segments();
        assert_eq!(segs.len(), 2);
        let alpha = segs.iter().find(|s| s.label == "alpha").expect("alpha segment");
        let beta = segs.iter().find(|s| s.label == "beta").expect("beta segment");
        assert_eq!(alpha.sample_count, 2);
        assert_eq!(beta.sample_count, 1);
        // avg alpha = (10+30)/2 = 20
        assert_eq!(alpha.avg_us, 20);
        assert_eq!(beta.avg_us, 20);
    }

    #[test]
    fn slowest_segment_returns_highest_avg() {
        let mut p = LatencyProfile::new();
        p.record("fast", 10);
        p.record("slow", 500);
        p.record("medium", 100);
        let slowest = p.slowest_segment().expect("should have a slowest");
        assert_eq!(slowest.label, "slow");
        assert_eq!(slowest.avg_us, 500);
    }

    #[test]
    fn total_avg_sums_all_segments() {
        let mut p = LatencyProfile::new();
        p.record("a", 100);
        p.record("b", 200);
        p.record("c", 300);
        assert_eq!(p.total_avg_us(), 600);
    }

    #[test]
    fn ring_buffer_eviction_keeps_last_1024_samples() {
        let mut p = LatencyProfile::new();
        // Record 2048 values: 1..=2048.  Only the last 1024 should remain in
        // the ring, but min/max/total_count should reflect all samples.
        for v in 1..=2048u64 {
            p.record("evict", v);
        }
        let segs = p.segments();
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        // sample_count tracks total, not just ring length
        assert_eq!(s.sample_count, 2048);
        // min/max are tracked globally
        assert_eq!(s.min_us, 1);
        assert_eq!(s.max_us, 2048);
        // avg is total/total_count = (2048*2049/2)/2048 = 1024.5 -> 1024
        let expected_avg = (1u64 + 2048) * 2048 / 2 / 2048;
        assert_eq!(s.avg_us, expected_avg);
        // p50 should come from the last 1024 samples (1025..=2048)
        // sorted values in ring: 1025..=2048
        // p50 of 1024 values: nearest-rank index = ceil(50/100 * 1024) - 1 = 511
        // sorted[511] = 1025 + 511 = 1536
        assert_eq!(s.p50_us, 1536);
    }

    #[test]
    fn saturating_add_prevents_overflow_on_total_avg() {
        let mut p = LatencyProfile::new();
        // Create many segments with large averages to stress total_avg_us.
        // Each segment with a single huge value.
        for i in 0..300u64 {
            let label: &'static str = Box::leak(format!("seg_{i}").into_boxed_str());
            p.record(label, u64::MAX / 2);
        }
        // total_avg_us should saturate rather than panic/overflow
        let total = p.total_avg_us();
        assert_eq!(total, u64::MAX, "saturating add should clamp to u64::MAX");
    }

    #[test]
    fn percentile_empty_returns_zero() {
        let sorted: &[u64] = &[];
        assert_eq!(percentile(sorted, 50), 0);
        assert_eq!(percentile(sorted, 99), 0);
    }

    #[test]
    fn percentile_single_element_returns_that_element() {
        let sorted: &[u64] = &[42];
        assert_eq!(percentile(sorted, 50), 42);
        assert_eq!(percentile(sorted, 99), 42);
        assert_eq!(percentile(sorted, 0), 42);
        assert_eq!(percentile(sorted, 100), 42);
    }

    #[test]
    fn record_zero_duration_works() {
        let mut p = LatencyProfile::new();
        p.record("zero", 0);
        let s = &p.segments()[0];
        assert_eq!(s.min_us, 0);
        assert_eq!(s.max_us, 0);
        assert_eq!(s.avg_us, 0);
        assert_eq!(s.p50_us, 0);
        assert_eq!(s.p99_us, 0);
    }

    #[test]
    fn full_pipeline_profile_example() {
        let mut p = LatencyProfile::new();
        // Simulate the canonical hot path from the design doc.
        p.record("submit -> admit", 300);
        p.record("submit -> admit", 350);
        p.record("admit -> first step", 100);
        p.record("admit -> first step", 120);
        p.record("first step -> action scheduled", 12_000);
        p.record("first step -> action scheduled", 13_000);
        p.record("action scheduled -> completed", 3_200_000);
        p.record("action scheduled -> completed", 3_300_000);
        p.record("completed -> finish", 200);
        p.record("completed -> finish", 250);

        let segs = p.segments();
        assert_eq!(segs.len(), 5);

        let slowest = p.slowest_segment().expect("must have slowest");
        assert_eq!(slowest.label, "action scheduled -> completed");

        // total_avg = 325 + 110 + 12500 + 3250000 + 225 = 3263160 us
        assert_eq!(p.total_avg_us(), 3_263_160);
    }
}
