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
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            hours, mins, secs, ms,
        )
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
        let mut entries: Vec<TimelineEntry> = records
            .iter()
            .map(TimelineEntry::from_record)
            .collect();
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
            make_record(1, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 3_000_000),
            make_record(2, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 1_000_000),
            make_record(3, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 2_000_000),
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
            make_record(10, 0, IncidentSeverity::Critical, FailureCode::TaintLeak, ReplaySafety::Safe, 1_000_000),
            make_record(20, 0, IncidentSeverity::Warning, FailureCode::ActionTimeout, ReplaySafety::Safe, 2_000_000),
            make_record(10, 1, IncidentSeverity::Major, FailureCode::BudgetExceeded, ReplaySafety::Safe, 3_000_000),
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
            make_record(1, 0, IncidentSeverity::Critical, FailureCode::TaintLeak, ReplaySafety::Safe, 1_000_000),
            make_record(2, 0, IncidentSeverity::Warning, FailureCode::ActionTimeout, ReplaySafety::Safe, 2_000_000),
            make_record(3, 0, IncidentSeverity::Critical, FailureCode::StepPanicked, ReplaySafety::UnsafeSideEffect, 3_000_000),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let criticals = timeline.filter_by_severity(IncidentSeverity::Critical);
        assert_eq!(criticals.entries.len(), 2);
        assert!(criticals.entries.iter().all(|e| e.severity == IncidentSeverity::Critical));
    }

    #[test]
    fn test_critical_count() {
        let records = vec![
            make_record(1, 0, IncidentSeverity::Critical, FailureCode::TaintLeak, ReplaySafety::Safe, 1_000_000),
            make_record(2, 0, IncidentSeverity::Warning, FailureCode::ActionTimeout, ReplaySafety::Safe, 2_000_000),
            make_record(3, 0, IncidentSeverity::Critical, FailureCode::StepPanicked, ReplaySafety::Safe, 3_000_000),
            make_record(4, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 4_000_000),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert_eq!(timeline.critical_count(), 2);
    }

    #[test]
    fn test_has_unsafe_replay_true() {
        let records = vec![
            make_record(1, 0, IncidentSeverity::Warning, FailureCode::ActionTimeout, ReplaySafety::Safe, 1_000_000),
            make_record(2, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::UnsafeSideEffect, 2_000_000),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert!(timeline.has_unsafe_replay());
    }

    #[test]
    fn test_has_unsafe_replay_false_all_safe() {
        let records = vec![
            make_record(1, 0, IncidentSeverity::Critical, FailureCode::TaintLeak, ReplaySafety::Safe, 1_000_000),
            make_record(2, 0, IncidentSeverity::Warning, FailureCode::ActionTimeout, ReplaySafety::Safe, 2_000_000),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        assert!(!timeline.has_unsafe_replay());
    }

    #[test]
    fn test_duration_ms_calculation() {
        let records = vec![
            make_record(1, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 1_500_000),
            make_record(2, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 4_500_000),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        // 4.5M us - 1.5M us = 3M us = 3000 ms
        assert_eq!(timeline.duration_ms(), 3000);
    }

    #[test]
    fn test_filter_by_run_empty_result() {
        let records = vec![
            make_record(1, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 1_000_000),
        ];
        let timeline = IncidentTimeline::from_records(&records);
        let filtered = timeline.filter_by_run(999);
        assert!(filtered.entries.is_empty());
        assert_eq!(filtered.earliest_us, 0);
        assert_eq!(filtered.latest_us, 0);
    }

    #[test]
    fn test_filter_by_severity_no_match() {
        let records = vec![
            make_record(1, 0, IncidentSeverity::Info, FailureCode::ActionTimeout, ReplaySafety::Safe, 1_000_000),
        ];
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
}
