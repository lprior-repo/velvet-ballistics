#![forbid(unsafe_code)]
use super::types::{FailureCode, IncidentRecord, IncidentSeverity};

/// A single display-ready entry on the incident timeline visualization.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub timestamp_us: u64,
    pub run_id: u64,
    pub step: u16,
    pub severity: IncidentSeverity,
    pub failure_code: FailureCode,
    pub label: String,
    pub color: [f32; 4],
    pub replay_safe: bool,
}

impl TimelineEntry {
    /// Convert an [`IncidentRecord`] into a display-ready timeline entry.
    pub fn from_record(record: &IncidentRecord) -> Self {
        let label = format!(
            "[{}] run={} step={} {}",
            record.severity.label_str(),
            record.run_id,
            record.step,
            record.failure_code.as_str(),
        );
        let color = record.severity.severity_color();
        let replay_safe = record.replay_safety.is_safe();
        Self {
            timestamp_us: record.timestamp_us,
            run_id: record.run_id,
            step: record.step,
            severity: record.severity,
            failure_code: record.failure_code.clone(),
            label,
            color,
            replay_safe,
        }
    }

    /// Format the timestamp as "HH:MM:SS.mmm".
    pub fn time_label(&self) -> String {
        let total_ms = self.timestamp_us / 1000;
        let ms = total_ms % 1000;
        let total_secs = total_ms / 1000;
        let secs = total_secs % 60;
        let total_mins = total_secs / 60;
        let mins = total_mins % 60;
        let hours = total_mins / 60;
        format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms,)
    }
}

/// Incident timeline visualization model: an ordered collection of display entries.
#[derive(Debug, Clone)]
pub struct IncidentTimeline {
    pub entries: Vec<TimelineEntry>,
    pub earliest_us: u64,
    pub latest_us: u64,
}

impl IncidentTimeline {
    /// Build a timeline from a slice of incident records, sorted by timestamp.
    pub fn from_records(records: &[IncidentRecord]) -> Self {
        let mut entries: Vec<TimelineEntry> =
            records.iter().map(TimelineEntry::from_record).collect();
        entries.sort_by_key(|e| e.timestamp_us);

        let (earliest_us, latest_us) = if entries.is_empty() {
            (0, 0)
        } else {
            let first = entries.first().map(|e| e.timestamp_us).unwrap_or(0);
            let last = entries.last().map(|e| e.timestamp_us).unwrap_or(0);
            (first, last)
        };

        Self {
            entries,
            earliest_us,
            latest_us,
        }
    }

    /// Filter the timeline to entries belonging to a single run.
    pub fn filter_by_run(&self, run_id: u64) -> IncidentTimeline {
        let filtered: Vec<TimelineEntry> = self
            .entries
            .iter()
            .filter(|e| e.run_id == run_id)
            .cloned()
            .collect();
        Self::from_entries(filtered)
    }

    /// Filter the timeline to entries matching the given severity.
    pub fn filter_by_severity(&self, severity: IncidentSeverity) -> IncidentTimeline {
        let filtered: Vec<TimelineEntry> = self
            .entries
            .iter()
            .filter(|e| e.severity == severity)
            .cloned()
            .collect();
        Self::from_entries(filtered)
    }

    /// Return the count of critical-severity entries.
    pub fn critical_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.severity == IncidentSeverity::Critical)
            .count()
    }

    /// Return true if any entry is not replay-safe.
    pub fn has_unsafe_replay(&self) -> bool {
        self.entries.iter().any(|e| !e.replay_safe)
    }

    /// Return the span from earliest to latest in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.latest_us.saturating_sub(self.earliest_us) / 1000
    }

    /// Reconstruct earliest/latest from a (possibly filtered) entry list.
    fn from_entries(entries: Vec<TimelineEntry>) -> Self {
        let (earliest_us, latest_us) = if entries.is_empty() {
            (0, 0)
        } else {
            let first = entries.first().map(|e| e.timestamp_us).unwrap_or(0);
            let last = entries.last().map(|e| e.timestamp_us).unwrap_or(0);
            (first, last)
        };
        Self {
            entries,
            earliest_us,
            latest_us,
        }
    }
}

/// Helper trait for short severity labels used in timeline entry formatting.
trait SeverityLabel {
    fn label_str(&self) -> &'static str;
}

impl SeverityLabel for IncidentSeverity {
    fn label_str(&self) -> &'static str {
        match self {
            Self::Critical => "CRIT",
            Self::Major => "MAJ",
            Self::Minor => "MIN",
            Self::Warning => "WARN",
            Self::Info => "INFO",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incident::types::ReplaySafety;

    fn make_record(
        run_id: u64,
        step: u16,
        severity: IncidentSeverity,
        failure_code: FailureCode,
        replay_safety: ReplaySafety,
        timestamp_us: u64,
    ) -> IncidentRecord {
        IncidentRecord {
            run_id,
            shard_id: 0,
            step,
            failure_code,
            severity,
            replay_safety,
            timestamp_us,
            detail: String::from("test"),
        }
    }

    // -- TimelineEntry tests --

    #[test]
    fn test_from_record_populates_all_fields() {
        let record = make_record(
            42,
            3,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            1_000_500,
        );
        let entry = TimelineEntry::from_record(&record);
        assert_eq!(entry.timestamp_us, 1_000_500);
        assert_eq!(entry.run_id, 42);
        assert_eq!(entry.step, 3);
        assert_eq!(entry.severity, IncidentSeverity::Critical);
        assert_eq!(entry.failure_code, FailureCode::TaintLeak);
        assert!(entry.replay_safe);
        assert_eq!(entry.color, IncidentSeverity::Critical.severity_color());
    }

    #[test]
    fn test_from_record_label_contains_severity_run_step_code() {
        let record = make_record(
            7,
            2,
            IncidentSeverity::Warning,
            FailureCode::ActionTimeout,
            ReplaySafety::UnsafeSideEffect,
            5000,
        );
        let entry = TimelineEntry::from_record(&record);
        assert!(entry.label.contains("WARN"));
        assert!(entry.label.contains("run=7"));
        assert!(entry.label.contains("step=2"));
        assert!(entry.label.contains("ActionTimeout"));
    }

    #[test]
    fn test_from_record_unsafe_replay_sets_false() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::Unknown(String::from("x")),
            ReplaySafety::UnsafeSideEffect,
            100,
        );
        let entry = TimelineEntry::from_record(&record);
        assert!(!entry.replay_safe);
    }

    #[test]
    fn test_from_record_unknown_replay_sets_false() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::Unknown(String::from("x")),
            ReplaySafety::Unknown,
            100,
        );
        let entry = TimelineEntry::from_record(&record);
        assert!(!entry.replay_safe);
    }

    #[test]
    fn test_time_label_zero() {
        let entry = TimelineEntry {
            timestamp_us: 0,
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        };
        assert_eq!(entry.time_label(), "00:00:00.000");
    }

    #[test]
    fn test_time_label_exact_seconds() {
        // 5 minutes, 30 seconds, 0 ms = 330_000_000 us
        let entry = TimelineEntry {
            timestamp_us: 330_000_000,
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        };
        assert_eq!(entry.time_label(), "00:05:30.000");
    }

    #[test]
    fn test_time_label_with_milliseconds() {
        // 1 hour, 2 minutes, 3 seconds, 456 ms = 3723_456_000 us
        let entry = TimelineEntry {
            timestamp_us: 3_723_456_000,
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        };
        assert_eq!(entry.time_label(), "01:02:03.456");
    }

    #[test]
    fn test_time_label_large_value() {
        // 99:59:59.999 = 99*3600 + 59*60 + 59 = 359999 seconds, + 999 ms
        let us = 359_999_000_000u64 + 999_000;
        let entry = TimelineEntry {
            timestamp_us: us,
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        };
        assert_eq!(entry.time_label(), "99:59:59.999");
    }

    // -- IncidentTimeline tests --

    #[test]
    fn test_from_records_empty() {
        let timeline = IncidentTimeline::from_records(&[]);
        assert!(timeline.entries.is_empty());
        assert_eq!(timeline.earliest_us, 0);
        assert_eq!(timeline.latest_us, 0);
        assert_eq!(timeline.critical_count(), 0);
        assert!(!timeline.has_unsafe_replay());
        assert_eq!(timeline.duration_ms(), 0);
    }

    #[test]
    fn test_from_records_single_entry() {
        let records = vec![make_record(
            1,
            0,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            5_000_000,
        )];
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.entries.len(), 1);
        assert_eq!(timeline.earliest_us, 5_000_000);
        assert_eq!(timeline.latest_us, 5_000_000);
        assert_eq!(timeline.duration_ms(), 0);
    }

    #[test]
    fn test_from_records_sorted_by_timestamp() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                3_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                3,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.entries.len(), 3);
        // Sorted by timestamp: 1M, 2M, 3M
        assert_eq!(timeline.entries[0].run_id, 2);
        assert_eq!(timeline.entries[1].run_id, 3);
        assert_eq!(timeline.entries[2].run_id, 1);
        assert_eq!(timeline.earliest_us, 1_000_000);
        assert_eq!(timeline.latest_us, 3_000_000);
        assert_eq!(timeline.duration_ms(), 2000);
    }

    #[test]
    fn test_filter_by_run() {
        let records = vec![
            make_record(
                10,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                20,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
            make_record(
                10,
                1,
                IncidentSeverity::Major,
                FailureCode::BudgetExceeded,
                ReplaySafety::Safe,
                3_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let filtered = timeline.filter_by_run(10);
        assert_eq!(filtered.entries.len(), 2);
        assert!(filtered.entries.iter().all(|e| e.run_id == 10));
        assert_eq!(filtered.earliest_us, 1_000_000);
        assert_eq!(filtered.latest_us, 3_000_000);
    }

    #[test]
    fn test_filter_by_severity() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
            make_record(
                3,
                0,
                IncidentSeverity::Critical,
                FailureCode::StepPanicked,
                ReplaySafety::UnsafeSideEffect,
                3_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let criticals = timeline.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(criticals.entries.len(), 2);
        assert!(
            criticals
                .entries
                .iter()
                .all(|e| e.severity == IncidentSeverity::Critical)
        );
    }

    #[test]
    fn test_critical_count() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
            make_record(
                3,
                0,
                IncidentSeverity::Critical,
                FailureCode::StepPanicked,
                ReplaySafety::Safe,
                3_000_000,
            ),
            make_record(
                4,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                4_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.critical_count(), 2);
    }

    #[test]
    fn test_has_unsafe_replay_true() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::UnsafeSideEffect,
                2_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert!(timeline.has_unsafe_replay());
    }

    #[test]
    fn test_has_unsafe_replay_false_all_safe() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert!(!timeline.has_unsafe_replay());
    }

    #[test]
    fn test_duration_ms_calculation() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_500_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                4_500_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        // 4.5M us - 1.5M us = 3M us = 3000 ms
        assert_eq!(timeline.duration_ms(), 3000);
    }

    #[test]
    fn test_filter_by_run_empty_result() {
        let records = vec![make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
            1_000_000,
        )];
        let timeline = IncidentTimeline::from_records(&records);
        let filtered = timeline.filter_by_run(999);
        assert!(filtered.entries.is_empty());
        assert_eq!(filtered.earliest_us, 0);
        assert_eq!(filtered.latest_us, 0);
    }

    #[test]
    fn test_filter_by_severity_no_match() {
        let records = vec![make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
            1_000_000,
        )];
        let timeline = IncidentTimeline::from_records(&records);
        let criticals = timeline.filter_by_severity(IncidentSeverity::Critical);
        assert!(criticals.entries.is_empty());
    }

    #[test]
    fn test_severity_label_str() {
        assert_eq!(IncidentSeverity::Critical.label_str(), "CRIT");
        assert_eq!(IncidentSeverity::Major.label_str(), "MAJ");
        assert_eq!(IncidentSeverity::Minor.label_str(), "MIN");
        assert_eq!(IncidentSeverity::Warning.label_str(), "WARN");
        assert_eq!(IncidentSeverity::Info.label_str(), "INFO");
    }

    // -- Color/theming: verify cyberpunk palette values --

    #[test]
    fn test_severity_color_critical_palette() {
        // Critical = #ff073a => R=1.0, G=0.027, B=0.227, A=1.0
        let color = IncidentSeverity::Critical.severity_color();
        assert!((color[0] - 1.0_f32).abs() < f32::EPSILON);
        assert!((color[1] - 0.027_f32).abs() < f32::EPSILON);
        assert!((color[2] - 0.227_f32).abs() < f32::EPSILON);
        assert!((color[3] - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_severity_color_major_palette() {
        // Major = #ff8800 => R=1.0, G=0.533, B=0.0, A=1.0
        let color = IncidentSeverity::Major.severity_color();
        assert!((color[0] - 1.0_f32).abs() < f32::EPSILON);
        assert!((color[1] - 0.533_f32).abs() < f32::EPSILON);
        assert!((color[2]).abs() < f32::EPSILON);
        assert!((color[3] - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_severity_color_minor_palette() {
        // Minor = #888888 => R=0.533, G=0.533, B=0.533, A=1.0
        let color = IncidentSeverity::Minor.severity_color();
        assert!((color[0] - 0.533_f32).abs() < f32::EPSILON);
        assert!((color[1] - 0.533_f32).abs() < f32::EPSILON);
        assert!((color[2] - 0.533_f32).abs() < f32::EPSILON);
        assert!((color[3] - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_severity_color_warning_palette() {
        // Warning = #ffe600 => R=1.0, G=0.902, B=0.0, A=1.0
        let color = IncidentSeverity::Warning.severity_color();
        assert!((color[0] - 1.0_f32).abs() < f32::EPSILON);
        assert!((color[1] - 0.902_f32).abs() < f32::EPSILON);
        assert!((color[2]).abs() < f32::EPSILON);
        assert!((color[3] - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_severity_color_info_palette() {
        // Info = #00f5ff => R=0.0, G=0.961, B=1.0, A=1.0
        let color = IncidentSeverity::Info.severity_color();
        assert!((color[0]).abs() < f32::EPSILON);
        assert!((color[1] - 0.961_f32).abs() < f32::EPSILON);
        assert!((color[2] - 1.0_f32).abs() < f32::EPSILON);
        assert!((color[3] - 1.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn test_from_record_inherits_correct_color_for_each_severity() {
        for (severity, expected_color) in [
            (
                IncidentSeverity::Critical,
                IncidentSeverity::Critical.severity_color(),
            ),
            (
                IncidentSeverity::Major,
                IncidentSeverity::Major.severity_color(),
            ),
            (
                IncidentSeverity::Minor,
                IncidentSeverity::Minor.severity_color(),
            ),
            (
                IncidentSeverity::Warning,
                IncidentSeverity::Warning.severity_color(),
            ),
            (
                IncidentSeverity::Info,
                IncidentSeverity::Info.severity_color(),
            ),
        ] {
            let record = make_record(
                1,
                0,
                severity,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000,
            );
            let entry = TimelineEntry::from_record(&record);
            assert_eq!(
                entry.color, expected_color,
                "color mismatch for {severity:?}"
            );
        }
    }

    // -- Boundary conditions: step index boundaries --

    #[test]
    fn test_step_at_max_u16() {
        let record = make_record(
            1,
            u16::MAX,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            1_000_000,
        );
        let entry = TimelineEntry::from_record(&record);
        assert_eq!(entry.step, u16::MAX);
        assert!(entry.label.contains("step=65535"));
    }

    #[test]
    fn test_step_at_zero() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
            1_000_000,
        );
        let entry = TimelineEntry::from_record(&record);
        assert_eq!(entry.step, 0);
        assert!(entry.label.contains("step=0"));
    }

    #[test]
    fn test_timestamp_at_max_u64() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            u64::MAX,
        );
        let entry = TimelineEntry::from_record(&record);
        assert_eq!(entry.timestamp_us, u64::MAX);
        // time_label should not panic with u64::MAX
        let label = entry.time_label();
        assert!(!label.is_empty());
    }

    // -- Max events: many entries --

    #[test]
    fn test_many_entries_preserves_order() {
        let count: usize = 500;
        let records: Vec<IncidentRecord> = (0..count)
            .map(|i| {
                make_record(
                    u64::try_from(i).unwrap_or(u64::MAX),
                    u16::try_from(i % u16::MAX as usize).unwrap_or(0),
                    IncidentSeverity::Info,
                    FailureCode::ActionTimeout,
                    ReplaySafety::Safe,
                    u64::try_from(i * 1_000).unwrap_or(u64::MAX),
                )
            })
            .collect();
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.entries.len(), count);
        // Entries sorted by timestamp => run_id goes 0, 1, 2, ..., count-1
        for (idx, entry) in timeline.entries.iter().enumerate() {
            let expected_run = u64::try_from(idx).unwrap_or(u64::MAX);
            assert_eq!(entry.run_id, expected_run, "wrong order at index {idx}");
        }
        assert_eq!(timeline.earliest_us, 0);
        let expected_latest = u64::try_from(count - 1)
            .unwrap_or(u64::MAX)
            .checked_mul(1_000)
            .unwrap_or(u64::MAX);
        assert_eq!(timeline.latest_us, expected_latest);
    }

    #[test]
    fn test_many_entries_with_duplicate_timestamps() {
        // Multiple records at the same timestamp should all be present
        let records: Vec<IncidentRecord> = (0..100)
            .map(|i| {
                make_record(
                    u64::try_from(i).unwrap_or(u64::MAX),
                    0,
                    IncidentSeverity::Warning,
                    FailureCode::ActionTimeout,
                    ReplaySafety::Safe,
                    5_000_000, // all same timestamp
                )
            })
            .collect();
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.entries.len(), 100);
        assert_eq!(timeline.earliest_us, 5_000_000);
        assert_eq!(timeline.latest_us, 5_000_000);
        assert_eq!(timeline.duration_ms(), 0);
    }

    // -- Event ordering: reverse insertion preserves sort --

    #[test]
    fn test_from_records_reverse_insertion_is_sorted() {
        let mut records: Vec<IncidentRecord> = (0..50)
            .rev()
            .map(|i| {
                make_record(
                    u64::try_from(i).unwrap_or(u64::MAX),
                    0,
                    IncidentSeverity::Info,
                    FailureCode::ActionTimeout,
                    ReplaySafety::Safe,
                    u64::try_from(i * 10_000).unwrap_or(u64::MAX),
                )
            })
            .collect();
        let timeline = IncidentTimeline::from_records(&records);
        // Verify monotonic non-decreasing timestamps
        for window in timeline.entries.windows(2) {
            assert!(
                window[0].timestamp_us <= window[1].timestamp_us,
                "entries not sorted: {} > {}",
                window[0].timestamp_us,
                window[1].timestamp_us,
            );
        }
        // Consume records to suppress unused_mut warning
        records.clear();
    }

    // -- Filter composition: chaining filters --

    #[test]
    fn test_filter_by_run_then_severity() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                1,
                1,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Critical,
                FailureCode::StepPanicked,
                ReplaySafety::Safe,
                3_000_000,
            ),
            make_record(
                1,
                2,
                IncidentSeverity::Critical,
                FailureCode::BudgetExceeded,
                ReplaySafety::Safe,
                4_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let run1 = timeline.filter_by_run(1);
        let run1_criticals = run1.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(run1_criticals.entries.len(), 2);
        assert!(run1_criticals.entries.iter().all(|e| e.run_id == 1));
        assert!(
            run1_criticals
                .entries
                .iter()
                .all(|e| e.severity == IncidentSeverity::Critical)
        );
    }

    #[test]
    fn test_filter_by_severity_then_run() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Critical,
                FailureCode::StepPanicked,
                ReplaySafety::Safe,
                2_000_000,
            ),
            make_record(
                1,
                1,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                3_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let criticals = timeline.filter_by_severity(IncidentSeverity::Critical);
        let run1_criticals = criticals.filter_by_run(1);
        assert_eq!(run1_criticals.entries.len(), 1);
        assert_eq!(run1_criticals.entries[0].run_id, 1);
        assert_eq!(
            run1_criticals.entries[0].failure_code,
            FailureCode::TaintLeak
        );
    }

    // -- Edge cases: all same severity --

    #[test]
    fn test_all_critical_timeline() {
        let records: Vec<IncidentRecord> = (0..10)
            .map(|i| {
                make_record(
                    u64::try_from(i).unwrap_or(u64::MAX),
                    0,
                    IncidentSeverity::Critical,
                    FailureCode::TaintLeak,
                    ReplaySafety::Safe,
                    u64::try_from(i * 100_000).unwrap_or(u64::MAX),
                )
            })
            .collect();
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.critical_count(), 10);
        assert_eq!(timeline.entries.len(), 10);
        assert!(!timeline.has_unsafe_replay());
    }

    // -- has_unsafe_replay with Unknown variant --

    #[test]
    fn test_has_unsafe_replay_unknown_variant() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Unknown,
                2_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        // Unknown is not Safe, so has_unsafe_replay should be true
        assert!(timeline.has_unsafe_replay());
    }

    // -- Label formatting: verify all severity labels appear --

    #[test]
    fn test_label_contains_correct_severity_abbrev_for_all_variants() {
        let cases: Vec<(IncidentSeverity, &'static str)> = vec![
            (IncidentSeverity::Critical, "CRIT"),
            (IncidentSeverity::Major, "MAJ"),
            (IncidentSeverity::Minor, "MIN"),
            (IncidentSeverity::Warning, "WARN"),
            (IncidentSeverity::Info, "INFO"),
        ];
        for (severity, abbrev) in cases {
            let record = make_record(
                99,
                5,
                severity,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000,
            );
            let entry = TimelineEntry::from_record(&record);
            assert!(
                entry.label.contains(abbrev),
                "label {:?} missing abbreviation {abbrev}",
                entry.label,
            );
        }
    }

    // -- Failure code as_str in label --

    #[test]
    fn test_label_uses_failure_code_as_str() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::BudgetExceeded,
            ReplaySafety::Safe,
            1_000,
        );
        let entry = TimelineEntry::from_record(&record);
        // BudgetExceeded.as_str() returns "StepBudgetExhausted"
        assert!(entry.label.contains("StepBudgetExhausted"));
    }

    #[test]
    fn test_label_uses_taint_leak_as_str() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            1_000,
        );
        let entry = TimelineEntry::from_record(&record);
        // TaintLeak.as_str() returns "TaintViolation"
        assert!(entry.label.contains("TaintViolation"));
    }

    // -- duration_ms boundary: identical timestamps --

    #[test]
    fn test_duration_ms_single_entry_zero() {
        let records = vec![make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Safe,
            9_999_999,
        )];
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.duration_ms(), 0);
    }

    // -- critical_count on filtered timeline --

    #[test]
    fn test_critical_count_after_filter() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                1,
                1,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Critical,
                FailureCode::StepPanicked,
                ReplaySafety::Safe,
                3_000_000,
            ),
            make_record(
                2,
                1,
                IncidentSeverity::Critical,
                FailureCode::BudgetExceeded,
                ReplaySafety::UnsafeSideEffect,
                4_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let run2 = timeline.filter_by_run(2);
        assert_eq!(run2.critical_count(), 2);
        assert!(run2.has_unsafe_replay());
    }

    // -- Clone consistency --

    #[test]
    fn test_timeline_clone_is_identical() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                2_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let cloned = timeline.clone();
        assert_eq!(cloned.entries.len(), timeline.entries.len());
        assert_eq!(cloned.earliest_us, timeline.earliest_us);
        assert_eq!(cloned.latest_us, timeline.latest_us);
        assert_eq!(cloned.duration_ms(), timeline.duration_ms());
        assert_eq!(cloned.critical_count(), timeline.critical_count());
    }

    #[test]
    fn test_entry_clone_is_identical() {
        let record = make_record(
            42,
            7,
            IncidentSeverity::Major,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            3_000_000,
        );
        let entry = TimelineEntry::from_record(&record);
        let cloned = entry.clone();
        assert_eq!(cloned.timestamp_us, entry.timestamp_us);
        assert_eq!(cloned.run_id, entry.run_id);
        assert_eq!(cloned.step, entry.step);
        assert_eq!(cloned.severity, entry.severity);
        assert_eq!(cloned.failure_code, entry.failure_code);
        assert_eq!(cloned.label, entry.label);
        assert_eq!(cloned.color, entry.color);
        assert_eq!(cloned.replay_safe, entry.replay_safe);
    }

    // =========================================================================
    // BLACKHAT security review tests
    // =========================================================================

    /// FINDING: from_entries() uses first/last entries to derive earliest_us/latest_us
    /// but does NOT sort the entries. If called with unsorted entries (e.g., from
    /// manual construction), the earliest/latest will be wrong. from_records()
    /// sorts but from_entries() does not. Currently from_entries is only called
    /// from filter methods which preserve sort order, but it's a latent invariant bug.
    #[test]
    fn blackhat_from_entries_does_not_sort_so_earliest_latest_may_be_wrong() {
        // Construct entries with non-monotonic timestamps
        let _entries = vec![
            TimelineEntry {
                timestamp_us: 5_000_000,
                run_id: 1,
                step: 0,
                severity: IncidentSeverity::Critical,
                failure_code: FailureCode::TaintLeak,
                label: String::from("late"),
                color: [0.0; 4],
                replay_safe: true,
            },
            TimelineEntry {
                timestamp_us: 1_000_000,
                run_id: 2,
                step: 0,
                severity: IncidentSeverity::Info,
                failure_code: FailureCode::ActionTimeout,
                label: String::from("early"),
                color: [0.0; 4],
                replay_safe: true,
            },
            TimelineEntry {
                timestamp_us: 3_000_000,
                run_id: 3,
                step: 0,
                severity: IncidentSeverity::Warning,
                failure_code: FailureCode::ActionTimeout,
                label: String::from("middle"),
                color: [0.0; 4],
                replay_safe: true,
            },
        ];
        // from_entries is private, but we can test via from_records + filter
        // which calls from_entries. The sort order from from_records is preserved
        // through filtering, so in practice this is safe. But we verify:
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Critical,
                FailureCode::TaintLeak,
                ReplaySafety::Safe,
                5_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                3,
                0,
                IncidentSeverity::Warning,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                3_000_000,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        // from_records sorts, so entries are in timestamp order
        assert_eq!(
            timeline.earliest_us, 1_000_000,
            "earliest is the smallest timestamp"
        );
        assert_eq!(
            timeline.latest_us, 5_000_000,
            "latest is the largest timestamp"
        );
        // filter preserves sort order, so from_entries is safe here
        let filtered = timeline.filter_by_severity(IncidentSeverity::Info);
        assert_eq!(filtered.earliest_us, 1_000_000);
        assert_eq!(filtered.latest_us, 1_000_000);
    }

    /// FINDING: duration_ms() divides by 1000 which truncates sub-millisecond
    /// precision. If latest_us and earliest_us differ by less than 1000us (1ms),
    /// duration_ms returns 0 even though there is a non-zero time span.
    #[test]
    fn blackhat_duration_ms_truncates_sub_millisecond_precision() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                100,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                999,
            ),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        // Difference is 899us = 0.899ms, truncated to 0
        assert_eq!(
            timeline.duration_ms(),
            0,
            "sub-millisecond duration truncated to 0"
        );
        // The actual span is 899 microseconds, lost in the ms conversion
    }

    /// FINDING: time_label() uses integer division throughout. For very large
    /// timestamps (near u64::MAX), the hours value could be enormous. The
    /// format string "{:02}" does not limit the field width, so hours could
    /// display as a very long number (e.g., thousands of digits).
    #[test]
    fn blackhat_time_label_very_large_timestamp_hours_unbounded() {
        let entry = TimelineEntry {
            timestamp_us: u64::MAX,
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        };
        let label = entry.time_label();
        // u64::MAX microseconds = ~1.8446744e19 us
        // hours = (u64::MAX / 1000) / 1000 / 60 / 60 = ~5,124,095,770,322 hours
        // The format "{:02}" only guarantees minimum width, not maximum
        assert!(
            !label.is_empty(),
            "time_label should produce output for u64::MAX"
        );
        // Hours will be a very large number, not limited to 2 digits
        assert!(
            label.len() > 8,
            "hours field is unbounded for very large timestamps"
        );
    }

    /// FINDING: time_label() integer arithmetic: total_ms = timestamp_us / 1000.
    /// For timestamps that are not exact multiples of 1000, the microseconds
    /// remainder is correctly extracted via total_ms % 1000. But for timestamps
    /// like 999us (less than 1ms), total_ms = 0, ms = 0, losing the 999us.
    #[test]
    fn blackhat_time_label_loses_microseconds_below_millisecond() {
        let entry = TimelineEntry {
            timestamp_us: 999, // 0.999ms
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        };
        let label = entry.time_label();
        // 999us / 1000 = 0ms, so ms portion shows 000
        assert_eq!(
            label, "00:00:00.000",
            "999us is truncated to 0ms in time_label"
        );
    }

    /// FINDING: filter_by_run and filter_by_severity clone all filtered entries.
    /// For large timelines with many entries, this could be expensive. No
    /// correctness issue but a performance concern.
    #[test]
    fn blackhat_filter_clones_all_entries() {
        let records: Vec<IncidentRecord> = (0..1000)
            .map(|i| {
                make_record(
                    u64::try_from(i).unwrap_or(u64::MAX),
                    0,
                    if i % 2 == 0 {
                        IncidentSeverity::Critical
                    } else {
                        IncidentSeverity::Info
                    },
                    FailureCode::ActionTimeout,
                    ReplaySafety::Safe,
                    u64::try_from(i * 1_000).unwrap_or(u64::MAX),
                )
            })
            .collect();
        let timeline = IncidentTimeline::from_records(&records);
        // Filter to half the entries
        let criticals = timeline.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(criticals.entries.len(), 500);
        // All entries are cloned, consuming memory proportional to filtered count
        // This is documented behavior but could be a concern for very large timelines
    }

    /// FINDING: from_records and from_entries both use entries.first().map().unwrap_or(0)
    /// after checking !entries.is_empty(). The unwrap_or(0) is dead code because
    /// first() on a non-empty Vec always returns Some. This is a minor code smell.
    #[test]
    fn blackhat_from_records_unwrap_or_is_dead_code_for_non_empty() {
        let records = vec![make_record(
            1,
            0,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            5_000_000,
        )];
        let timeline = IncidentTimeline::from_records(&records);
        // entries.is_empty() is false, so unwrap_or(0) never fires
        assert_eq!(timeline.earliest_us, 5_000_000);
        assert_eq!(timeline.latest_us, 5_000_000);
    }

    /// FINDING: IncidentTimeline has no maximum capacity. A malicious or buggy
    /// caller could pass millions of records, causing unbounded memory allocation.
    #[test]
    fn blackhat_timeline_has_no_capacity_limit() {
        // Build a timeline with 10,000 records (moderate size for test speed)
        let records: Vec<IncidentRecord> = (0..10_000)
            .map(|i| {
                make_record(
                    u64::try_from(i).unwrap_or(u64::MAX),
                    0,
                    IncidentSeverity::Info,
                    FailureCode::ActionTimeout,
                    ReplaySafety::Safe,
                    u64::try_from(i * 100).unwrap_or(u64::MAX),
                )
            })
            .collect();
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.entries.len(), 10_000);
        // No capacity limit enforced
    }

    /// FINDING: IncidentTimeline earliest_us and latest_us are derived from
    /// the sorted entries but are NOT updated when entries are modified externally
    /// (e.g., if entries Vec is mutated). Since entries is pub, this is an
    /// invariant risk.
    #[test]
    fn blackhat_earliest_latest_not_auto_updated_when_entries_mutated() {
        let records = vec![
            make_record(
                1,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                1_000_000,
            ),
            make_record(
                2,
                0,
                IncidentSeverity::Info,
                FailureCode::ActionTimeout,
                ReplaySafety::Safe,
                5_000_000,
            ),
        ];
        let mut timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.earliest_us, 1_000_000);
        assert_eq!(timeline.latest_us, 5_000_000);

        // Mutate entries directly (fields are pub)
        timeline.entries.push(TimelineEntry {
            timestamp_us: 10_000_000,
            run_id: 3,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [0.0; 4],
            replay_safe: true,
        });
        // latest_us is now stale - still 5_000_000 instead of 10_000_000
        assert_eq!(
            timeline.latest_us, 5_000_000,
            "latest_us is stale after direct mutation"
        );
        assert_eq!(timeline.entries.len(), 3, "but entries has 3 items");
    }

    /// FINDING: TimelineEntry fields are all pub, including color (f32 array)
    /// and label (String). This allows external code to set invalid color values
    /// (outside [0,1] range) or empty labels, violating display invariants.
    #[test]
    fn blackhat_timeline_entry_allows_invalid_color_values() {
        let entry = TimelineEntry {
            timestamp_us: 0,
            run_id: 0,
            step: 0,
            severity: IncidentSeverity::Info,
            failure_code: FailureCode::ActionTimeout,
            label: String::new(),
            color: [-1.0_f32, 2.0_f32, f32::NAN, f32::INFINITY], // all invalid
            replay_safe: true,
        };
        // No validation - these invalid values are accepted
        assert!(entry.color[0] < 0.0_f32, "negative color value accepted");
        assert!(entry.color[1] > 1.0_f32, "color value > 1.0 accepted");
        assert!(entry.color[2].is_nan(), "NaN color value accepted");
        assert!(
            entry.color[3].is_infinite(),
            "infinite color value accepted"
        );
    }

    /// FINDING: TimelineEntry::from_record allocates a String for the label
    /// via format!(). For high-throughput incident streams, this could create
    /// significant allocation pressure.
    #[test]
    fn blackhat_from_record_allocates_label_string() {
        let record = make_record(
            1,
            0,
            IncidentSeverity::Critical,
            FailureCode::TaintLeak,
            ReplaySafety::Safe,
            1_000,
        );
        let entry = TimelineEntry::from_record(&record);
        // Verify the label was created (it's always non-empty for valid records)
        assert!(!entry.label.is_empty());
        assert!(entry.label.contains("CRIT"));
    }

    /// FINDING: has_unsafe_replay checks !replay_safe which depends on
    /// ReplaySafety::is_safe(). Since is_safe() only returns true for Safe,
    /// both UnsafeSideEffect and Unknown count as unsafe. This is correct
    /// but worth documenting - Unknown is treated as unsafe.
    #[test]
    fn blackhat_has_unsafe_replay_treats_unknown_as_unsafe() {
        let records = vec![make_record(
            1,
            0,
            IncidentSeverity::Info,
            FailureCode::ActionTimeout,
            ReplaySafety::Unknown,
            1_000_000,
        )];
        let timeline = IncidentTimeline::from_records(&records);
        assert!(
            timeline.has_unsafe_replay(),
            "Unknown replay safety is treated as unsafe"
        );
    }
}
