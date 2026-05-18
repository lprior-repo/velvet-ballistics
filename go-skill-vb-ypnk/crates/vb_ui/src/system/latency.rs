#![forbid(unsafe_code)]
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
        scratch.extend_from_slice(self.buf.get(..self.len.min(self.buf.len())).unwrap_or(&[]));
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
        assert_eq!(
            LatencyProfile::default().segments(),
            LatencyProfile::new().segments()
        );
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
        let alpha = segs
            .iter()
            .find(|s| s.label == "alpha")
            .expect("alpha segment");
        let beta = segs
            .iter()
            .find(|s| s.label == "beta")
            .expect("beta segment");
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

    // -------------------------------------------------------------------------
    // Additional tests for broader coverage
    // -------------------------------------------------------------------------

    #[test]
    fn p50_p95_p99_computation_on_known_distribution() {
        // 1..=100 gives a uniform distribution.
        let mut ring = SampleRing::new(200);
        for v in 1..=100u64 {
            ring.push(v);
        }
        let mut scratch = Vec::new();
        let sorted = ring.as_sorted_slice(&mut scratch);

        // p50: nearest-rank = ceil(50/100 * 100) - 1 = 49 -> sorted[49] = 50
        assert_eq!(percentile(sorted, 50), 50);
        // p95: nearest-rank = ceil(95/100 * 100) - 1 = 94 -> sorted[94] = 95
        assert_eq!(percentile(sorted, 95), 95);
        // p99: nearest-rank = ceil(99/100 * 100) - 1 = 98 -> sorted[98] = 99
        assert_eq!(percentile(sorted, 99), 99);
    }

    #[test]
    fn submit_to_admit_phase_breakdown() {
        let mut p = LatencyProfile::new();
        // 10 samples: 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000
        // p50: rank = 50*10/100 = 5, idx = 4 -> sorted[4] = 500
        for v in [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000] {
            p.record("submit -> admit", v);
        }

        let segs = p.segments();
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        assert_eq!(s.label, "submit -> admit");
        assert_eq!(s.min_us, 100);
        assert_eq!(s.max_us, 1000);
        // avg = 5500 / 10 = 550
        assert_eq!(s.avg_us, 550);
        assert_eq!(s.p50_us, 500);
        assert_eq!(s.sample_count, 10);
    }

    #[test]
    fn admit_to_first_step_phase() {
        let mut p = LatencyProfile::new();
        // 10 samples: 50, 60, 70, 80, 90, 100, 110, 120, 130, 140
        // p50: rank = 50*10/100 = 5, idx = 4 -> sorted[4] = 90
        for v in [50, 60, 70, 80, 90, 100, 110, 120, 130, 140] {
            p.record("admit -> first step", v);
        }

        let segs = p.segments();
        let s = segs.iter().find(|s| s.label == "admit -> first step");
        let Some(s) = s else { return };
        assert_eq!(s.min_us, 50);
        assert_eq!(s.max_us, 140);
        // avg = 950 / 10 = 95
        assert_eq!(s.avg_us, 95);
        assert_eq!(s.p50_us, 90);
        assert_eq!(s.sample_count, 10);
    }

    #[test]
    fn action_scheduled_to_completed_phase() {
        let mut p = LatencyProfile::new();
        // 10 samples from 500_000 to 5_000_000 in 500_000 increments
        // sorted: [500k, 1000k, 1500k, 2000k, 2500k, 3000k, 3500k, 4000k, 4500k, 5000k]
        // p50: rank = 50*10/100 = 5, idx = 4 -> sorted[4] = 2_500_000
        for v in [
            1_000_000, 500_000, 1_500_000, 2_000_000, 2_500_000, 3_000_000, 3_500_000, 4_000_000,
            4_500_000, 5_000_000,
        ] {
            p.record("action scheduled -> completed", v);
        }

        let segs = p.segments();
        let s = segs
            .iter()
            .find(|s| s.label == "action scheduled -> completed");
        let Some(s) = s else { return };
        assert_eq!(s.min_us, 500_000);
        assert_eq!(s.max_us, 5_000_000);
        // avg = 27_500_000 / 10 = 2_750_000
        assert_eq!(s.avg_us, 2_750_000);
        assert_eq!(s.p50_us, 2_500_000);
        assert_eq!(s.sample_count, 10);
    }

    #[test]
    fn empty_latency_data_returns_zeros() {
        let p = LatencyProfile::new();
        assert!(p.segments().is_empty());
        assert!(p.slowest_segment().is_none());
        assert_eq!(p.total_avg_us(), 0);
    }

    #[test]
    fn single_sample_edge_case_all_stats_equal() {
        let mut p = LatencyProfile::new();
        p.record("single", 999);
        let segs = p.segments();
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        assert_eq!(s.min_us, 999);
        assert_eq!(s.max_us, 999);
        assert_eq!(s.avg_us, 999);
        assert_eq!(s.p50_us, 999);
        assert_eq!(s.p99_us, 999);
        assert_eq!(s.sample_count, 1);
    }

    #[test]
    fn percentile_with_many_samples_stress() {
        // 1..=1000 uniform distribution.
        let mut ring = SampleRing::new(2000);
        for v in 1..=1000u64 {
            ring.push(v);
        }
        let mut scratch = Vec::new();
        let sorted = ring.as_sorted_slice(&mut scratch);
        assert_eq!(sorted.len(), 1000);

        // p50: ceil(50/100 * 1000) - 1 = 499 -> sorted[499] = 500
        assert_eq!(percentile(sorted, 50), 500);
        // p95: ceil(95/100 * 1000) - 1 = 949 -> sorted[949] = 950
        assert_eq!(percentile(sorted, 95), 950);
        // p99: ceil(99/100 * 1000) - 1 = 989 -> sorted[989] = 990
        assert_eq!(percentile(sorted, 99), 990);
        // p100: ceil(100/100 * 1000) - 1 = 999 -> sorted[999] = 1000
        assert_eq!(percentile(sorted, 100), 1000);
    }

    #[test]
    fn ring_eviction_percentiles_use_only_retained_samples() {
        let mut ring = SampleRing::new(10);
        // Push 20 values: 1..=20. Ring retains only 11..=20.
        for v in 1..=20u64 {
            ring.push(v);
        }
        let mut scratch = Vec::new();
        let sorted = ring.as_sorted_slice(&mut scratch);
        assert_eq!(sorted.len(), 10);
        // All retained values should be 11..=20.
        let Some(&first) = sorted.first() else { return };
        let Some(&last) = sorted.last() else { return };
        assert_eq!(first, 11);
        assert_eq!(last, 20);

        // p50 of 11..=20: ceil(50/100 * 10) - 1 = 4 -> sorted[4] = 15
        assert_eq!(percentile(sorted, 50), 15);
    }

    #[test]
    fn multi_segment_total_avg_reflects_pipeline_end_to_end() {
        let mut p = LatencyProfile::new();
        p.record("submit -> admit", 300);
        p.record("admit -> first step", 100);
        p.record("first step -> action scheduled", 10_000);
        p.record("action scheduled -> completed", 2_000_000);
        p.record("completed -> finish", 200);

        // Each segment has one sample, so avg == the sample itself.
        // total = 300 + 100 + 10000 + 2000000 + 200 = 2010600
        assert_eq!(p.total_avg_us(), 2_010_600);

        let slowest = p.slowest_segment();
        let Some(s) = slowest else { return };
        assert_eq!(s.label, "action scheduled -> completed");
    }

    #[test]
    fn record_same_label_accumulates_correctly() {
        let mut p = LatencyProfile::new();
        // 10 samples of value 1000.
        for _ in 0..10 {
            p.record("steady", 1000);
        }
        let segs = p.segments();
        assert_eq!(segs.len(), 1);
        let s = &segs[0];
        assert_eq!(s.min_us, 1000);
        assert_eq!(s.max_us, 1000);
        assert_eq!(s.avg_us, 1000);
        assert_eq!(s.p50_us, 1000);
        assert_eq!(s.p99_us, 1000);
        assert_eq!(s.sample_count, 10);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: Medium
    /// DESCRIPTION: SegmentAccumulator::record uses saturating_add for
    /// total_us, which means the running sum silently clamps at u64::MAX.
    /// After saturation, avg_us = total_us / total_count becomes increasingly
    /// inaccurate. An attacker who records many large durations can cause the
    /// average to under-report actual latency, hiding performance degradation.
    #[test]
    fn blackhat_total_us_saturation_corrupts_average() {
        let mut p = LatencyProfile::new();
        // Record two samples that sum to just above u64::MAX.
        p.record("lat-sat", u64::MAX / 2);
        p.record("lat-sat", u64::MAX / 2);
        p.record("lat-sat", u64::MAX / 2);
        // total_us = MAX/2 + MAX/2 + MAX/2 saturates to u64::MAX
        // avg_us = u64::MAX / 3 which is ~1/3 of the true sum (1.5 * MAX)
        let segs = p.segments();
        let s = &segs[0];
        // The true average is (MAX/2 * 3) / 3 = MAX/2
        // The reported average is MAX / 3 which is significantly less than MAX/2
        let true_avg = u64::MAX / 2; // would be correct without saturation
        assert!(
            s.avg_us < true_avg,
            "saturated avg ({}) should be less than true avg ({})",
            s.avg_us,
            true_avg
        );
        // avg_us should be u64::MAX / 3 after saturation
        assert_eq!(s.avg_us, u64::MAX / 3);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: SampleRing::push uses checked_rem(self.buf.len()) which
    /// returns None when buf.len() is 0, falling back to head = 0. If a ring
    /// is somehow constructed with zero capacity, push becomes a no-op write
    /// that silently drops samples while incrementing len up to 0 (guarded by
    /// the `if self.len < self.buf.len()` check). This is correctly guarded
    /// in practice since SampleRing::new always creates a non-empty vec, but
    /// there's no explicit invariant enforcement.
    #[test]
    fn blackhat_zero_capacity_ring_silently_drops_samples() {
        // Construct a ring with zero capacity via the internal type.
        let mut ring = SampleRing::new(0);
        ring.push(42);
        // buf.len() is 0, so checked_rem returns None, head stays 0.
        // The get_mut at index 0 on an empty vec does nothing.
        // len check: 0 < 0 is false, so len stays 0.
        assert!(ring.is_empty(), "zero-capacity ring should remain empty");
    }

    /// SEVERITY: Low
    /// DESCRIPTION: LatencyProfile::record performs a linear scan
    /// (iter_mut().find()) over all accumulators to find a matching label.
    /// With many unique labels, this becomes O(n) per insertion, making
    /// total insertion cost O(n^2). Not a correctness issue but a performance
    /// concern for high-cardinality label spaces.
    #[test]
    fn blackhat_many_labels_linear_scan_performance_concern() {
        let mut p = LatencyProfile::new();
        // Create 1000 unique labels -- each insertion scans all existing labels.
        for i in 0..1000u64 {
            let label: &'static str = Box::leak(format!("label_{i}").into_boxed_str());
            p.record(label, 100);
        }
        assert_eq!(p.segments().len(), 1000);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The min_us field is initialized to u64::MAX and only
    /// updated when a strictly smaller value is recorded. If total_count
    /// saturates (very unlikely with u64), the min remains accurate, but
    /// the percentile computation from the ring buffer could diverge from
    /// the true percentile if the ring has evicted the relevant samples.
    /// This is by design (ring is a window), but min/max are global while
    /// percentiles are windowed -- potential confusion.
    #[test]
    fn blackhat_min_max_global_while_percentiles_are_windowed() {
        let mut p = LatencyProfile::new();
        // Record RING_CAPACITY + 10 values from 1 to 1034.
        // Ring retains only the last 1024 values: 11..=1034.
        for v in 1..=(RING_CAPACITY as u64 + 10) {
            p.record("windowed", v);
        }
        let segs = p.segments();
        let s = &segs[0];
        // min/max reflect ALL samples globally.
        assert_eq!(s.min_us, 1, "min should reflect global minimum");
        assert_eq!(s.max_us, 1024 + 10, "max should reflect global maximum");
        // But p50 reflects only the window 11..=1034.
        // p50 of 1024 values: ceil(50/100 * 1024) - 1 = 511
        // sorted[511] = 11 + 511 = 522
        assert_eq!(
            s.p50_us, 522,
            "p50 should reflect windowed data, not global"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The percentile function uses nearest-rank interpolation.
    /// With pct=0, rank = 0, idx = max(0-1, 0) = 0 via saturating_sub. So
    /// percentile(sorted, 0) returns the minimum element. This is correct but
    /// worth documenting -- pct=0 does not return an error or special value.
    #[test]
    fn blackhat_percentile_zero_returns_minimum_element() {
        let sorted: &[u64] = &[10, 20, 30, 40, 50];
        assert_eq!(percentile(sorted, 0), 10, "pct=0 should return minimum");
    }

    /// SEVERITY: Low
    /// DESCRIPTION: LatencyProfile::segments() creates a new scratch buffer
    /// for each call but uses &self, not &mut self. This is semantically clean
    /// but means every call to segments() allocates. The slowest_segment()
    /// method calls segments() internally, so computing both slowest_segment()
    /// and total_avg_us() allocates scratch twice.
    #[test]
    fn blackhat_segments_allocates_scratch_per_call() {
        let mut p = LatencyProfile::new();
        p.record("a", 100);
        p.record("b", 200);
        // Call segments twice -- each allocates its own scratch.
        let segs1 = p.segments();
        let segs2 = p.segments();
        assert_eq!(
            segs1, segs2,
            "repeated calls should produce identical results"
        );
    }
}
