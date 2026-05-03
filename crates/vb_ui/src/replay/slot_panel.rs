//! Slot diff panel -- shows what changed at each replay event boundary.
//!
//! Phase 1D component: computes and renders slot-value diffs between two
//! replay states, supporting both single-event inspection (via
//! [`SlotDiffPanel::from_event`]) and full state comparison (via
//! [`SlotDiffPanel::diff_between`]).

use std::collections::HashMap;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;
use vb_storage::events::JournalEvent;

/// Describes the kind of change observed for a single slot.
///
/// Stores formatted [`String`] representations rather than borrowed
/// [`SlotValue`] references, so the diff is fully owned and can outlive
/// the source data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotDiff {
    /// Slot appeared with a new value (was absent in the previous state).
    Created(String),
    /// Slot value changed from `old` to `new`.
    Modified {
        /// Formatted previous value.
        old: String,
        /// Formatted new value.
        new: String,
    },
    /// Slot was removed (present in previous state, absent in new state).
    Deleted(String),
    /// Slot value did not change but its taint label did.
    TaintChanged {
        /// Formatted previous taint.
        old: String,
        /// Formatted new taint.
        new: String,
    },
}

/// A single slot diff entry: which slot, and what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    /// Slot that changed.
    pub slot: SlotIdx,
    /// Description of the change.
    pub diff: SlotDiff,
}

/// Panel model for displaying slot diffs at a replay boundary.
pub struct SlotDiffPanel {
    entries: Vec<DiffEntry>,
    event_seq: u32,
}

impl SlotDiffPanel {
    /// Creates an empty panel (no entries, seq = 0).
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            event_seq: 0,
        }
    }

    /// Build a panel from a single [`JournalEvent::SlotWrittenEvent`].
    ///
    /// For `SlotWrittenEvent` variants, records the slot write as
    /// [`SlotDiff::Created`] (slot absent from `current_slots`) or
    /// [`SlotDiff::Modified`] (slot present with a different value).
    /// All other event variants produce an empty panel.
    #[must_use]
    pub fn from_event(
        event: &JournalEvent,
        current_slots: &HashMap<SlotIdx, SlotValue>,
    ) -> Self {
        match event {
            JournalEvent::SlotWrittenEvent {
                seq, slot, value, ..
            } => {
                let new_value: Option<SlotValue> = match value {
                    Some(bytes) => postcard::from_bytes(bytes).ok(),
                    None => None,
                };
                let seq_val = seq.get();

                let Some(new_val) = new_value else {
                    return Self {
                        entries: Vec::new(),
                        event_seq: u32::try_from(seq_val).unwrap_or(u32::MAX),
                    };
                };

                let new_fmt = format!("{new_val:?}");

                let diff = match current_slots.get(slot) {
                    None => SlotDiff::Created(new_fmt),
                    Some(old_val) => {
                        let old_fmt = format!("{old_val:?}");
                        if old_fmt == new_fmt {
                            return Self {
                                entries: Vec::new(),
                                event_seq: u32::try_from(seq_val).unwrap_or(u32::MAX),
                            };
                        }
                        SlotDiff::Modified {
                            old: old_fmt,
                            new: new_fmt,
                        }
                    }
                };

                let seq_u32 = u32::try_from(seq_val).unwrap_or(u32::MAX);
                Self {
                    entries: vec![DiffEntry {
                        slot: *slot,
                        diff,
                    }],
                    event_seq: seq_u32,
                }
            }
            _ => Self::new(),
        }
    }

    /// Compute all differences between two slot-state snapshots.
    #[must_use]
    pub fn diff_between(
        before: &HashMap<SlotIdx, SlotValue>,
        after: &HashMap<SlotIdx, SlotValue>,
    ) -> Self {
        let mut entries = Vec::new();

        for (&slot, new_val) in after {
            let new_fmt = format!("{new_val:?}");
            match before.get(&slot) {
                None => {
                    entries.push(DiffEntry {
                        slot,
                        diff: SlotDiff::Created(new_fmt),
                    });
                }
                Some(old_val) => {
                    let old_fmt = format!("{old_val:?}");
                    if old_fmt != new_fmt {
                        entries.push(DiffEntry {
                            slot,
                            diff: SlotDiff::Modified {
                                old: old_fmt,
                                new: new_fmt,
                            },
                        });
                    }
                }
            }
        }

        for (&slot, old_val) in before {
            if after.get(&slot).is_none() {
                entries.push(DiffEntry {
                    slot,
                    diff: SlotDiff::Deleted(format!("{old_val:?}")),
                });
            }
        }

        Self {
            entries,
            event_seq: 0,
        }
    }

    /// Returns all diff entries in the panel.
    #[must_use]
    pub fn entries(&self) -> &[DiffEntry] {
        &self.entries
    }

    /// Returns `true` if the panel contains at least one diff entry.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Returns the event sequence number associated with this panel.
    #[must_use]
    pub const fn event_seq(&self) -> u32 {
        self.event_seq
    }

    /// Returns a human-readable diff line for a single entry.
    #[must_use]
    pub fn format_entry(entry: &DiffEntry) -> String {
        let slot_label = format!("SlotIdx({})", entry.slot.get());
        match &entry.diff {
            SlotDiff::Created(val) => {
                format!("{slot_label}: <created> {val}")
            }
            SlotDiff::Modified { old, new } => {
                format!("{slot_label}: {old} -> {new}")
            }
            SlotDiff::Deleted(val) => {
                format!("{slot_label}: {val} -> <deleted>")
            }
            SlotDiff::TaintChanged { old, new } => {
                format!("{slot_label}: taint {old} -> {new}")
            }
        }
    }
}

impl Default for SlotDiffPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::ObjectId;
    use vb_storage::types::EventSeq;

    fn slot_map(pairs: &[(u16, SlotValue)]) -> HashMap<SlotIdx, SlotValue> {
        pairs.iter().map(|(k, v)| (SlotIdx::new(*k), *v)).collect()
    }

    fn make_slot_written_event(slot: u16, value: SlotValue, seq: u64) -> JournalEvent {
        let bytes = postcard::to_allocvec(&value);
        JournalEvent::SlotWrittenEvent {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(seq),
            slot: SlotIdx::new(slot),
            value: bytes.ok(),
        }
    }

    fn make_step_started_event(seq: u64) -> JournalEvent {
        JournalEvent::StepStarted {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(seq),
            step: vb_core::ids::StepIdx::new(0),
        }
    }

    #[test]
    fn new_panel_is_empty() {
        let panel = SlotDiffPanel::new();
        assert!(panel.entries().is_empty());
        assert!(!panel.has_changes());
        assert_eq!(panel.event_seq(), 0);
    }

    #[test]
    fn default_matches_new() {
        let panel = SlotDiffPanel::default();
        assert!(panel.entries().is_empty());
        assert!(!panel.has_changes());
    }

    #[test]
    fn from_event_non_slot_event_produces_empty() {
        let event = make_step_started_event(5);
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(!panel.has_changes());
        assert!(panel.entries().is_empty());
    }

    #[test]
    fn from_event_slot_created_when_absent_from_current() {
        let event = make_slot_written_event(12, SlotValue::Null, 42);
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 1);
        assert_eq!(panel.event_seq(), 42);
        let entry = panel.entries().get(0).expect("entry exists");
        assert_eq!(entry.slot, SlotIdx::new(12));
        assert_eq!(entry.diff, SlotDiff::Created(String::from("Null")));
    }

    #[test]
    fn from_event_slot_modified_when_present_in_current() {
        let event = make_slot_written_event(5, SlotValue::I64(99), 10);
        let current = slot_map(&[(5, SlotValue::I64(1))]);
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 1);
        let entry = panel.entries().get(0).expect("entry exists");
        assert_eq!(entry.slot, SlotIdx::new(5));
        assert_eq!(
            entry.diff,
            SlotDiff::Modified {
                old: String::from("I64(1)"),
                new: String::from("I64(99)"),
            }
        );
    }

    #[test]
    fn from_event_no_diff_when_same_value() {
        let event = make_slot_written_event(3, SlotValue::Bool(true), 7);
        let current = slot_map(&[(3, SlotValue::Bool(true))]);
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(!panel.has_changes());
    }

    #[test]
    fn from_event_no_value_bytes_produces_empty() {
        let event = JournalEvent::SlotWrittenEvent {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(10),
            slot: SlotIdx::new(0),
            value: None,
        };
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(!panel.has_changes());
        assert_eq!(panel.event_seq(), 10);
    }

    #[test]
    fn diff_between_empty_states_produces_no_changes() {
        let before = HashMap::new();
        let after = HashMap::new();
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(!panel.has_changes());
    }

    #[test]
    fn diff_between_detects_created_slots() {
        let before = HashMap::new();
        let after = slot_map(&[(1, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 1);
        let entry = panel.entries().get(0).expect("entry");
        assert_eq!(entry.slot, SlotIdx::new(1));
        assert!(matches!(entry.diff, SlotDiff::Created(_)));
    }

    #[test]
    fn diff_between_detects_deleted_slots() {
        let before = slot_map(&[(7, SlotValue::Bool(false))]);
        let after = HashMap::new();
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 1);
        let entry = panel.entries().get(0).expect("entry");
        assert_eq!(entry.slot, SlotIdx::new(7));
        assert!(matches!(entry.diff, SlotDiff::Deleted(_)));
    }

    #[test]
    fn diff_between_detects_modified_slots() {
        let before = slot_map(&[(3, SlotValue::I64(10))]);
        let after = slot_map(&[(3, SlotValue::I64(20))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 1);
        let entry = panel.entries().get(0).expect("entry");
        assert_eq!(entry.slot, SlotIdx::new(3));
        assert_eq!(
            entry.diff,
            SlotDiff::Modified {
                old: String::from("I64(10)"),
                new: String::from("I64(20)"),
            }
        );
    }

    #[test]
    fn diff_between_no_changes_when_identical() {
        let before = slot_map(&[(2, SlotValue::Null), (4, SlotValue::Bool(true))]);
        let after = slot_map(&[(2, SlotValue::Null), (4, SlotValue::Bool(true))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(!panel.has_changes());
    }

    #[test]
    fn diff_between_multiple_changes() {
        let before = slot_map(&[
            (1, SlotValue::I64(10)),
            (2, SlotValue::Bool(true)),
            (3, SlotValue::Null),
        ]);
        let after = slot_map(&[
            (1, SlotValue::I64(99)),
            (3, SlotValue::Null),
            (5, SlotValue::Bool(false)),
        ]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 3);
        let slots: Vec<SlotIdx> = panel.entries().iter().map(|e| e.slot).collect();
        assert!(slots.contains(&SlotIdx::new(1)));
        assert!(slots.contains(&SlotIdx::new(2)));
        assert!(slots.contains(&SlotIdx::new(5)));
        for entry in panel.entries() {
            match entry.slot.get() {
                1 => {
                    assert_eq!(
                        entry.diff,
                        SlotDiff::Modified {
                            old: String::from("I64(10)"),
                            new: String::from("I64(99)"),
                        }
                    );
                }
                2 => {
                    assert_eq!(entry.diff, SlotDiff::Deleted(String::from("Bool(true)")));
                }
                5 => {
                    assert_eq!(entry.diff, SlotDiff::Created(String::from("Bool(false)")));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn format_entry_created() {
        let entry = DiffEntry {
            slot: SlotIdx::new(12),
            diff: SlotDiff::Created(String::from("Null")),
        };
        let result = SlotDiffPanel::format_entry(&entry);
        assert_eq!(result, "SlotIdx(12): <created> Null");
    }

    #[test]
    fn format_entry_modified() {
        let entry = DiffEntry {
            slot: SlotIdx::new(12),
            diff: SlotDiff::Modified {
                old: String::from("Null"),
                new: String::from("Object(ObjectId(8472))"),
            },
        };
        let result = SlotDiffPanel::format_entry(&entry);
        assert_eq!(result, "SlotIdx(12): Null -> Object(ObjectId(8472))");
    }

    #[test]
    fn format_entry_deleted() {
        let entry = DiffEntry {
            slot: SlotIdx::new(7),
            diff: SlotDiff::Deleted(String::from("Bool(true)")),
        };
        let result = SlotDiffPanel::format_entry(&entry);
        assert_eq!(result, "SlotIdx(7): Bool(true) -> <deleted>");
    }

    #[test]
    fn format_entry_taint_changed() {
        let entry = DiffEntry {
            slot: SlotIdx::new(4),
            diff: SlotDiff::TaintChanged {
                old: String::from("Clean"),
                new: String::from("Secret"),
            },
        };
        let result = SlotDiffPanel::format_entry(&entry);
        assert_eq!(result, "SlotIdx(4): taint Clean -> Secret");
    }

    #[test]
    fn from_event_seq_capped_to_u32_max() {
        let event = make_slot_written_event(0, SlotValue::I64(1), u64::from(u32::MAX) + 1);
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert_eq!(panel.event_seq(), u32::MAX);
    }

    #[test]
    fn slot_diff_equality_created() {
        let a = SlotDiff::Created(String::from("Null"));
        let b = SlotDiff::Created(String::from("Null"));
        assert_eq!(a, b);
    }

    #[test]
    fn slot_diff_inequality_created_vs_modified() {
        let a = SlotDiff::Created(String::from("Null"));
        let b = SlotDiff::Modified {
            old: String::from("Null"),
            new: String::from("Null"),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn from_event_object_value_formatting() {
        let event = make_slot_written_event(8, SlotValue::Object(ObjectId::new(8472)), 15);
        let current = slot_map(&[(8, SlotValue::Null)]);
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(panel.has_changes());
        let entry = panel.entries().get(0).expect("entry");
        let formatted = SlotDiffPanel::format_entry(entry);
        assert!(formatted.contains("Object(ObjectId(8472))"));
        assert!(formatted.contains("Null -> Object(ObjectId(8472))"));
    }
}
