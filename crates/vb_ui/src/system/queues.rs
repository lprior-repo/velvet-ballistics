#![forbid(unsafe_code)]
// Queue snapshot model for the System Overview "engine room" panel.
//
// Tracks aggregate queue depths across five pools -- ready, action,
// journal, trace ring, and frame pool -- and maps utilisation ratios
// to a three-band pressure scale (Normal / Warning / Critical) with
// corresponding display colours.
//
// Boundary thresholds (matching the project-wide convention):
// - Normal:   utilisation < 50%
// - Warning:  50% <= utilisation < 80%
// - Critical: utilisation >= 80%

// ---------------------------------------------------------------------------
// PressureLevel
// ---------------------------------------------------------------------------

/// Three-band pressure classification shared across all queue pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PressureLevel {
    /// Utilisation below 50%. Display colour: cyan `#00f5ff`.
    Normal,
    /// Utilisation at 50-79%. Display colour: yellow `#ffe600`.
    Warning,
    /// Utilisation at 80%+. Display colour: red `#ff073a`.
    Critical,
}

impl PressureLevel {
    /// Returns the RGBA display colour for this pressure level.
    ///
    /// | Level    | Hex       | RGBA (f32)                          |
    /// |----------|-----------|--------------------------------------|
    /// | Normal   | `#00f5ff` | `[0.0, 0.961, 1.0, 1.0]`           |
    /// | Warning  | `#ffe600` | `[1.0, 0.902, 0.0, 1.0]`           |
    /// | Critical | `#ff073a` | `[1.0, 0.027, 0.227, 1.0]`         |
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::Normal => [0.0, 0.961, 1.0, 1.0],
            Self::Warning => [1.0, 0.902, 0.0, 1.0],
            Self::Critical => [1.0, 0.027, 0.227, 1.0],
        }
    }

    /// Classify a `depth / capacity` ratio into a pressure band.
    ///
    /// Uses integer cross-multiplication to avoid floating-point
    /// imprecision at the 50% and 80% boundaries.
    ///
    /// Returns `Normal` when `capacity == 0` (no meaningful ratio).
    #[must_use]
    pub fn from_ratio(depth: u32, capacity: u32) -> Self {
        if capacity == 0 {
            return Self::Normal;
        }
        // depth / capacity >= 0.8  <=>  depth * 10 >= capacity * 8
        // depth / capacity >= 0.5  <=>  depth * 10 >= capacity * 5
        let depth_x10 = depth.saturating_mul(10);
        let cap_x8 = capacity.saturating_mul(8);
        if depth_x10 >= cap_x8 {
            Self::Critical
        } else {
            let cap_x5 = capacity.saturating_mul(5);
            if depth_x10 >= cap_x5 {
                Self::Warning
            } else {
                Self::Normal
            }
        }
    }
}

// ---------------------------------------------------------------------------
// QueueSnapshot
// ---------------------------------------------------------------------------

/// Point-in-time snapshot of all five queue pool depths for the
/// engine-room panel.
#[derive(Debug, Clone)]
pub struct QueueSnapshot {
    /// Ready-queue depth (commands waiting to be scheduled).
    pub ready_depth: u32,
    /// Ready-queue nominal capacity.
    pub ready_capacity: u32,
    /// Action-completion queue depth.
    pub action_depth: u32,
    /// Action-completion queue nominal capacity.
    pub action_capacity: u32,
    /// Journal writer queue depth.
    pub journal_depth: u32,
    /// Journal writer queue nominal capacity.
    pub journal_capacity: u32,
    /// Trace ring buffer fill percentage (0.0 - 100.0).
    pub trace_fill_pct: f32,
    /// Frame pool free count.
    pub frame_pool_free: u32,
    /// Frame pool total count.
    pub frame_pool_total: u32,
}

impl QueueSnapshot {
    /// Returns the worst pressure level across all five pools.
    ///
    /// Precedence: `Critical > Warning > Normal`.
    #[must_use]
    pub fn pressure_level(&self) -> PressureLevel {
        let candidates = [
            self.ready_pressure(),
            self.action_pressure(),
            self.journal_pressure(),
            self.trace_pressure(),
            self.frame_pressure(),
        ];
        let mut worst = PressureLevel::Normal;
        for level in candidates {
            if matches!(worst, PressureLevel::Normal)
                && matches!(level, PressureLevel::Warning | PressureLevel::Critical)
            {
                worst = level;
            }
            if matches!(level, PressureLevel::Critical) {
                worst = PressureLevel::Critical;
            }
        }
        worst
    }

    /// Pressure for the ready queue, based on `ready_depth / ready_capacity`.
    #[must_use]
    pub fn ready_pressure(&self) -> PressureLevel {
        PressureLevel::from_ratio(self.ready_depth, self.ready_capacity)
    }

    /// Pressure for the action-completion queue.
    #[must_use]
    pub fn action_pressure(&self) -> PressureLevel {
        PressureLevel::from_ratio(self.action_depth, self.action_capacity)
    }

    /// Pressure for the journal writer queue.
    #[must_use]
    pub fn journal_pressure(&self) -> PressureLevel {
        PressureLevel::from_ratio(self.journal_depth, self.journal_capacity)
    }

    /// Pressure for the trace ring buffer, derived from `trace_fill_pct`.
    #[must_use]
    pub fn trace_pressure(&self) -> PressureLevel {
        if self.trace_fill_pct >= 80.0 {
            PressureLevel::Critical
        } else if self.trace_fill_pct >= 50.0 {
            PressureLevel::Warning
        } else {
            PressureLevel::Normal
        }
    }

    /// Pressure for the frame pool, based on used / total ratio.
    ///
    /// "Used" is `frame_pool_total - frame_pool_free`. Returns `Normal`
    /// when `frame_pool_total == 0`.
    #[must_use]
    pub fn frame_pressure(&self) -> PressureLevel {
        let used = self.frame_pool_total.saturating_sub(self.frame_pool_free);
        PressureLevel::from_ratio(used, self.frame_pool_total)
    }

    /// Convenience: returns the RGBA colour for a given pressure level.
    ///
    /// This is a thin wrapper around [`PressureLevel::color`] kept on
    /// `QueueSnapshot` so callers don't need to import `PressureLevel`
    /// just to obtain a colour.
    #[must_use]
    pub fn pressure_color(level: PressureLevel) -> [f32; 4] {
        level.color()
    }
}

// ---------------------------------------------------------------------------
// Tests -- at least 10 covering pressure-level boundary transitions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- PressureLevel::color tests --

    #[test]
    fn pressure_level_normal_color_is_cyan() {
        let [r, g, b, a] = PressureLevel::Normal.color();
        assert_eq!(r, 0.0);
        assert!((g - 0.961).abs() < 0.002, "g={g}");
        assert_eq!(b, 1.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn pressure_level_warning_color_is_yellow() {
        let [r, g, b, a] = PressureLevel::Warning.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.902).abs() < 0.002, "g={g}");
        assert_eq!(b, 0.0);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn pressure_level_critical_color_is_red() {
        let [r, g, b, a] = PressureLevel::Critical.color();
        assert_eq!(r, 1.0);
        assert!((g - 0.027).abs() < 0.002, "g={g}");
        assert!((b - 0.227).abs() < 0.002, "b={b}");
        assert_eq!(a, 1.0);
    }

    // -- PressureLevel::from_ratio boundary tests --

    #[test]
    fn from_ratio_zero_capacity_returns_normal() {
        assert_eq!(PressureLevel::from_ratio(0, 0), PressureLevel::Normal);
        assert_eq!(PressureLevel::from_ratio(999, 0), PressureLevel::Normal);
    }

    #[test]
    fn from_ratio_below_50_pct_is_normal() {
        assert_eq!(PressureLevel::from_ratio(49, 100), PressureLevel::Normal);
        assert_eq!(PressureLevel::from_ratio(0, 100), PressureLevel::Normal);
    }

    #[test]
    fn from_ratio_exactly_50_pct_is_warning() {
        assert_eq!(PressureLevel::from_ratio(50, 100), PressureLevel::Warning);
    }

    #[test]
    fn from_ratio_79_pct_is_warning() {
        assert_eq!(PressureLevel::from_ratio(79, 100), PressureLevel::Warning);
    }

    #[test]
    fn from_ratio_exactly_80_pct_is_critical() {
        assert_eq!(PressureLevel::from_ratio(80, 100), PressureLevel::Critical);
    }

    #[test]
    fn from_ratio_100_pct_and_overflow_is_critical() {
        assert_eq!(PressureLevel::from_ratio(100, 100), PressureLevel::Critical);
        assert_eq!(PressureLevel::from_ratio(200, 100), PressureLevel::Critical);
    }

    // -- QueueSnapshot individual pressure methods --

    fn healthy_snapshot() -> QueueSnapshot {
        QueueSnapshot {
            ready_depth: 10,
            ready_capacity: 256,
            action_depth: 5,
            action_capacity: 256,
            journal_depth: 3,
            journal_capacity: 64,
            trace_fill_pct: 20.0,
            frame_pool_free: 90,
            frame_pool_total: 100,
        }
    }

    #[test]
    fn snapshot_healthy_all_normal() {
        let snap = healthy_snapshot();
        assert_eq!(snap.ready_pressure(), PressureLevel::Normal);
        assert_eq!(snap.action_pressure(), PressureLevel::Normal);
        assert_eq!(snap.journal_pressure(), PressureLevel::Normal);
        assert_eq!(snap.trace_pressure(), PressureLevel::Normal);
        assert_eq!(snap.frame_pressure(), PressureLevel::Normal);
        assert_eq!(snap.pressure_level(), PressureLevel::Normal);
    }

    #[test]
    fn snapshot_ready_at_50_pct_is_warning() {
        let mut snap = healthy_snapshot();
        snap.ready_depth = 128;
        snap.ready_capacity = 256;
        assert_eq!(snap.ready_pressure(), PressureLevel::Warning);
    }

    #[test]
    fn snapshot_ready_at_80_pct_is_critical() {
        let mut snap = healthy_snapshot();
        snap.ready_depth = 205; // 205/256 * 10 = 2050 >= 256*8 = 2048
        snap.ready_capacity = 256;
        assert_eq!(snap.ready_pressure(), PressureLevel::Critical);
    }

    #[test]
    fn snapshot_action_pressure_boundary() {
        let mut snap = healthy_snapshot();
        // action at 50%
        snap.action_depth = 128;
        snap.action_capacity = 256;
        assert_eq!(snap.action_pressure(), PressureLevel::Warning);

        // action at 80%
        snap.action_depth = 205;
        assert_eq!(snap.action_pressure(), PressureLevel::Critical);
    }

    #[test]
    fn snapshot_journal_pressure_boundary() {
        let mut snap = healthy_snapshot();
        // journal at 50%: 32/64
        snap.journal_depth = 32;
        snap.journal_capacity = 64;
        assert_eq!(snap.journal_pressure(), PressureLevel::Warning);

        // journal at 80%: 52/64 (520 >= 512)
        snap.journal_depth = 52;
        assert_eq!(snap.journal_pressure(), PressureLevel::Critical);
    }

    #[test]
    fn snapshot_trace_pressure_boundary() {
        let mut snap = healthy_snapshot();

        // trace just below 50%
        snap.trace_fill_pct = 49.9;
        assert_eq!(snap.trace_pressure(), PressureLevel::Normal);

        // trace at exactly 50%
        snap.trace_fill_pct = 50.0;
        assert_eq!(snap.trace_pressure(), PressureLevel::Warning);

        // trace at exactly 80%
        snap.trace_fill_pct = 80.0;
        assert_eq!(snap.trace_pressure(), PressureLevel::Critical);

        // trace above 80%
        snap.trace_fill_pct = 95.0;
        assert_eq!(snap.trace_pressure(), PressureLevel::Critical);
    }

    #[test]
    fn snapshot_frame_pressure_boundary() {
        let mut snap = healthy_snapshot();
        // frame pool: 50/100 used = 50% -> Warning
        snap.frame_pool_free = 50;
        snap.frame_pool_total = 100;
        assert_eq!(snap.frame_pressure(), PressureLevel::Warning);

        // frame pool: 80/100 used = 80% -> Critical
        snap.frame_pool_free = 20;
        assert_eq!(snap.frame_pressure(), PressureLevel::Critical);

        // frame pool: 0 total -> Normal (no meaningful ratio)
        snap.frame_pool_free = 0;
        snap.frame_pool_total = 0;
        assert_eq!(snap.frame_pressure(), PressureLevel::Normal);
    }

    #[test]
    fn snapshot_pressure_level_returns_worst_across_pools() {
        let mut snap = healthy_snapshot();
        // Make only the ready queue critical, everything else normal
        snap.ready_depth = 250;
        snap.ready_capacity = 256;
        assert_eq!(snap.pressure_level(), PressureLevel::Critical);
    }

    #[test]
    fn snapshot_pressure_level_warning_when_no_critical() {
        let mut snap = healthy_snapshot();
        // Ready at 50% -> Warning, everything else normal
        snap.ready_depth = 128;
        snap.ready_capacity = 256;
        assert_eq!(snap.pressure_level(), PressureLevel::Warning);
    }

    #[test]
    fn snapshot_pressure_color_delegates_to_level_color() {
        assert_eq!(
            QueueSnapshot::pressure_color(PressureLevel::Normal),
            PressureLevel::Normal.color()
        );
        assert_eq!(
            QueueSnapshot::pressure_color(PressureLevel::Warning),
            PressureLevel::Warning.color()
        );
        assert_eq!(
            QueueSnapshot::pressure_color(PressureLevel::Critical),
            PressureLevel::Critical.color()
        );
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// SEVERITY: HIGH
    /// DESCRIPTION: PressureLevel::from_ratio uses integer cross-multiplication
    /// with saturating_mul(10) to avoid floating-point. However, for large
    /// depth values, depth.saturating_mul(10) can saturate to u32::MAX, making
    /// the comparison unreliable. For depth=429_496_730 (u32::MAX/10 + 1),
    /// depth*10 = 4_294_967_300 which overflows u32 and saturates to MAX.
    /// With capacity=2_147_483_647 (u32::MAX/2), cap*8 also saturates.
    /// Both sides saturate to MAX -> equality -> Critical, but the true ratio
    /// is ~20% (Normal). This MISCLASSIFIES Normal load as Critical.
    #[test]
    fn blackhat_from_ratio_saturation_causes_false_critical() {
        // depth = u32::MAX / 10 + 1 = 429_496_730
        // depth * 10 = 4_294_967_300 > u32::MAX -> saturates to u32::MAX
        // capacity = u32::MAX / 2 = 2_147_483_647
        // True ratio = 429_496_730 / 2_147_483_647 ~= 0.2 (Normal)
        // cap_x8 = 2_147_483_647 * 8 -> overflows -> saturates to u32::MAX
        // depth_x10 (MAX) >= cap_x8 (MAX) is true -> Critical (WRONG!)
        let depth = u32::MAX / 10 + 1; // 429_496_730 -- triggers depth_x10 saturation
        let capacity = u32::MAX / 2; // 2_147_483_647 -- triggers cap_x8 saturation
        let result = PressureLevel::from_ratio(depth, capacity);
        // The true ratio is ~0.2, should be Normal.
        // But due to saturation, both sides equal u32::MAX.
        assert_ne!(
            result,
            PressureLevel::Normal,
            "BUG: ~20% utilization classified as non-Normal due to mul saturation"
        );
        assert_eq!(
            result,
            PressureLevel::Critical,
            "saturated comparison makes both sides equal -> Critical"
        );
    }

    /// SEVERITY: HIGH
    /// DESCRIPTION: With depth=500_000_000, capacity=1_000_000_000, the true
    /// ratio is exactly 50% (Warning). But depth_x10 = 5_000_000_000 which
    /// saturates to u32::MAX, and cap_x8 = 8_000_000_000 which also saturates
    /// to u32::MAX. depth_x10 >= cap_x8 is true (both MAX) -> Critical.
    /// This misclassifies a 50% utilization as Critical.
    #[test]
    fn blackhat_from_ratio_medium_values_boundary() {
        // depth=500_000_000, capacity=1_000_000_000 -> true ratio = 50%
        // depth_x10 = 5_000_000_000 -> saturates to u32::MAX
        // cap_x8 = 8_000_000_000 -> saturates to u32::MAX
        // depth_x10 >= cap_x8: MAX >= MAX -> true -> Critical (WRONG!)
        let depth = 500_000_000u32;
        let capacity = 1_000_000_000u32;
        let result = PressureLevel::from_ratio(depth, capacity);
        // True ratio is exactly 50%, should be Warning.
        assert_ne!(
            result,
            PressureLevel::Warning,
            "BUG: 50% ratio misclassified due to mul saturation"
        );
        assert_eq!(
            result,
            PressureLevel::Critical,
            "50% is classified as Critical due to saturated cross-multiply"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: QueueSnapshot::frame_pressure computes "used" as
    /// frame_pool_total.saturating_sub(frame_pool_free). If free > total
    /// (corrupt data), used = 0, and the pool appears Normal. This silently
    /// hides data corruption.
    #[test]
    fn blackhat_frame_pressure_free_exceeds_total_shows_normal() {
        let snap = QueueSnapshot {
            ready_depth: 0,
            ready_capacity: 100,
            action_depth: 0,
            action_capacity: 100,
            journal_depth: 0,
            journal_capacity: 100,
            trace_fill_pct: 10.0,
            frame_pool_free: 200, // corrupt: free > total
            frame_pool_total: 100,
        };
        // used = 100.saturating_sub(200) = 0
        // from_ratio(0, 100) = Normal
        assert_eq!(
            snap.frame_pressure(),
            PressureLevel::Normal,
            "corrupt free > total silently shows Normal"
        );
    }

    /// SEVERITY: Low
    /// DESCRIPTION: QueueSnapshot::pressure_level iterates through all five
    /// pool pressures and picks the worst. The logic uses a slightly convoluted
    /// match pattern that could miss the transition from Warning to Critical
    /// in certain orders. Testing confirms the logic is correct but the code
    /// is fragile and could break with refactoring.
    #[test]
    fn blackhat_pressure_level_worst_case_propagation() {
        // Test all combinations of individual pool pressures.
        let mut snap = QueueSnapshot {
            ready_depth: 0,
            ready_capacity: 100,
            action_depth: 0,
            action_capacity: 100,
            journal_depth: 0,
            journal_capacity: 100,
            trace_fill_pct: 10.0,
            frame_pool_free: 50, // 50/100 used = 50% -> Warning
            frame_pool_total: 100,
        };
        assert_eq!(snap.pressure_level(), PressureLevel::Warning);

        // Add a critical pool.
        snap.ready_depth = 90;
        snap.ready_capacity = 100;
        assert_eq!(snap.pressure_level(), PressureLevel::Critical);
    }

    /// SEVERITY: Low
    /// DESCRIPTION: The trace_fill_pct field is f32 and could be negative or
    /// exceed 100.0 (no validation). trace_pressure handles values >= 80 and
    /// >= 50, but negative values would be classified as Normal. An extremely
    /// large value (f32::INFINITY) would be Critical. NaN comparisons always
    /// return false, so NaN would be classified as Normal.
    #[test]
    fn blackhat_trace_pressure_handles_nan_and_negative() {
        let mut snap = QueueSnapshot {
            ready_depth: 0,
            ready_capacity: 100,
            action_depth: 0,
            action_capacity: 100,
            journal_depth: 0,
            journal_capacity: 100,
            trace_fill_pct: f32::NAN,
            frame_pool_free: 100,
            frame_pool_total: 100,
        };
        // NaN >= 80.0 is false, NaN >= 50.0 is false, so Normal.
        assert_eq!(
            snap.trace_pressure(),
            PressureLevel::Normal,
            "NaN trace_fill_pct should classify as Normal"
        );

        snap.trace_fill_pct = -10.0;
        assert_eq!(
            snap.trace_pressure(),
            PressureLevel::Normal,
            "negative trace_fill_pct should classify as Normal"
        );
    }
}
