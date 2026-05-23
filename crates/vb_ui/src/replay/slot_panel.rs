#![forbid(unsafe_code)]
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
#[non_exhaustive]
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
    pub fn from_event(event: &JournalEvent, current_slots: &HashMap<SlotIdx, SlotValue>) -> Self {
        match event {
            JournalEvent::SlotWrittenEvent {
                seq, slot, value, ..
            } => {
                let new_value: Option<SlotValue> = match value {
                    Some(bytes) => match postcard::from_bytes(bytes) {
                        Ok(decoded) => Some(decoded),
                        Err(_) => None,
                    },
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
                    entries: vec![DiffEntry { slot: *slot, diff }],
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
            extra: None,
            attempt: 1,
        }
    }

    fn make_step_started_event(seq: u64) -> JournalEvent {
        JournalEvent::StepStarted {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(seq),
            step: vb_core::ids::StepIdx::new(0),
            attempt: 1,
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
            extra: None,
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

    // -------------------------------------------------------------------------
    // Additional tests for coverage
    // -------------------------------------------------------------------------

    /// SlotDiffPanel built from `from_event` against an empty current map
    /// with `SlotValue::I64(0)` should record a Created diff.
    #[test]
    fn from_event_empty_slots_creates_zero_value() {
        let event = make_slot_written_event(0, SlotValue::I64(0), 1);
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 1);
        let Some(entry) = panel.entries().get(0) else {
            return;
        };
        assert_eq!(entry.slot, SlotIdx::new(0));
        assert_eq!(entry.diff, SlotDiff::Created(String::from("I64(0)")));
    }

    /// Multiple sequential `from_event` calls each produce independent panels;
    /// applying them to a progressively-updated slot map simulates a replay.
    #[test]
    fn from_event_multiple_slot_writes_progressive() {
        let mut current = HashMap::new();

        // First write: slot 10 created with Bool(true)
        let ev1 = make_slot_written_event(10, SlotValue::Bool(true), 100);
        let panel1 = SlotDiffPanel::from_event(&ev1, &current);
        assert!(panel1.has_changes());
        assert_eq!(panel1.event_seq(), 100);
        let Some(e1) = panel1.entries().get(0) else {
            return;
        };
        assert_eq!(e1.slot, SlotIdx::new(10));
        assert_eq!(e1.diff, SlotDiff::Created(String::from("Bool(true)")));

        // Update the current state
        current.insert(SlotIdx::new(10), SlotValue::Bool(true));

        // Second write: slot 10 modified to Bool(false)
        let ev2 = make_slot_written_event(10, SlotValue::Bool(false), 200);
        let panel2 = SlotDiffPanel::from_event(&ev2, &current);
        assert!(panel2.has_changes());
        let Some(e2) = panel2.entries().get(0) else {
            return;
        };
        assert_eq!(
            e2.diff,
            SlotDiff::Modified {
                old: String::from("Bool(true)"),
                new: String::from("Bool(false)"),
            }
        );

        // Third write: slot 20 created
        current.insert(SlotIdx::new(10), SlotValue::Bool(false));
        let ev3 = make_slot_written_event(20, SlotValue::Null, 300);
        let panel3 = SlotDiffPanel::from_event(&ev3, &current);
        assert!(panel3.has_changes());
        assert_eq!(panel3.entries().len(), 1);
        let Some(e3) = panel3.entries().get(0) else {
            return;
        };
        assert_eq!(e3.slot, SlotIdx::new(20));
        assert_eq!(e3.diff, SlotDiff::Created(String::from("Null")));
    }

    /// When `diff_between` is given multiple slots, the returned entries
    /// must contain exactly the slots that changed -- no duplicates, no extras.
    /// This verifies slot index correctness and that ordering is deterministic
    /// enough to find all expected slots.
    #[test]
    fn diff_between_slot_index_set_correctness() {
        let before = slot_map(&[
            (1, SlotValue::I64(10)),
            (3, SlotValue::I64(30)),
            (5, SlotValue::I64(50)),
        ]);
        let after = slot_map(&[
            (1, SlotValue::I64(11)),
            (3, SlotValue::I64(30)), // unchanged -- should NOT appear
            (7, SlotValue::I64(70)),
        ]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 3);

        let slots: Vec<u16> = panel.entries().iter().map(|e| e.slot.get()).collect();

        // Slot 1 modified, slot 5 deleted, slot 7 created
        assert!(slots.contains(&1));
        assert!(slots.contains(&5));
        assert!(slots.contains(&7));
        // Slot 3 did not change
        assert!(!slots.contains(&3));
    }

    /// `diff_between` must not report a diff when both states have the same
    /// value for the same slot, even when other slots differ.
    #[test]
    fn diff_between_same_value_no_diff_across_mixed_slots() {
        let before = slot_map(&[
            (2, SlotValue::Bool(false)),
            (4, SlotValue::I64(999)),
            (6, SlotValue::Null),
        ]);
        // Only slot 2 changes; slots 4 and 6 stay the same.
        let after = slot_map(&[
            (2, SlotValue::Bool(true)),
            (4, SlotValue::I64(999)),
            (6, SlotValue::Null),
        ]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert_eq!(panel.entries().len(), 1);
        let Some(entry) = panel.entries().get(0) else {
            return;
        };
        assert_eq!(entry.slot, SlotIdx::new(2));
        assert!(matches!(entry.diff, SlotDiff::Modified { .. }));
    }

    /// Using the maximum `SlotIdx` value (`u16::MAX`) should work correctly
    /// in both `from_event` and `diff_between`.
    #[test]
    fn boundary_max_slot_index() {
        let max_slot = u16::MAX;

        // from_event with max slot index
        let event = make_slot_written_event(max_slot, SlotValue::I64(42), 1);
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(panel.has_changes());
        let Some(entry) = panel.entries().get(0) else {
            return;
        };
        assert_eq!(entry.slot.get(), max_slot);

        // diff_between with max slot index
        let before = HashMap::new();
        let after = slot_map(&[(max_slot, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(panel.has_changes());
        let Some(entry) = panel.entries().get(0) else {
            return;
        };
        assert_eq!(entry.slot.get(), max_slot);
        assert_eq!(entry.diff, SlotDiff::Created(String::from("I64(42)")));
    }

    /// The `SlotDiff::TaintChanged` variant should format correctly and
    /// participate in equality checks, enabling taint propagation tracking
    /// at the diff-entry level.
    #[test]
    fn taint_changed_diff_entry_equality_and_formatting() {
        let entry_a = DiffEntry {
            slot: SlotIdx::new(3),
            diff: SlotDiff::TaintChanged {
                old: String::from("Clean"),
                new: String::from("Secret"),
            },
        };
        let entry_b = DiffEntry {
            slot: SlotIdx::new(3),
            diff: SlotDiff::TaintChanged {
                old: String::from("Clean"),
                new: String::from("Secret"),
            },
        };
        assert_eq!(entry_a, entry_b);

        let formatted = SlotDiffPanel::format_entry(&entry_a);
        assert_eq!(formatted, "SlotIdx(3): taint Clean -> Secret");

        // Verify inequality when new taint differs
        let entry_c = DiffEntry {
            slot: SlotIdx::new(3),
            diff: SlotDiff::TaintChanged {
                old: String::from("Clean"),
                new: String::from("Tainted"),
            },
        };
        assert_ne!(entry_a, entry_c);
    }

    /// A panel constructed manually with mixed `SlotDiff` variants
    /// (Created, Modified, Deleted, TaintChanged) reports `has_changes`
    /// and each entry formats without panic.
    #[test]
    fn mixed_diff_variants_panel_has_changes_and_formats() {
        let entries = vec![
            DiffEntry {
                slot: SlotIdx::new(1),
                diff: SlotDiff::Created(String::from("I64(7)")),
            },
            DiffEntry {
                slot: SlotIdx::new(2),
                diff: SlotDiff::Modified {
                    old: String::from("Bool(true)"),
                    new: String::from("Bool(false)"),
                },
            },
            DiffEntry {
                slot: SlotIdx::new(3),
                diff: SlotDiff::Deleted(String::from("Null")),
            },
            DiffEntry {
                slot: SlotIdx::new(4),
                diff: SlotDiff::TaintChanged {
                    old: String::from("Public"),
                    new: String::from("Private"),
                },
            },
        ];

        let panel = SlotDiffPanel {
            entries,
            event_seq: 55,
        };

        assert!(panel.has_changes());
        assert_eq!(panel.entries().len(), 4);
        assert_eq!(panel.event_seq(), 55);

        // Verify all entries format successfully
        for entry in panel.entries() {
            let formatted = SlotDiffPanel::format_entry(entry);
            assert!(!formatted.is_empty());
        }

        let Some(first_entry) = panel.entries().get(0) else {
            return;
        };
        let f0 = SlotDiffPanel::format_entry(first_entry);
        assert!(f0.contains("<created>"));
    }

    // =========================================================================
    // BLACK HAT security and correctness findings
    // =========================================================================

    /// FINDING 1 — MEDIUM: Value comparison via Debug format string.
    ///
    /// `from_event` and `diff_between` compare slot values by formatting them
    /// with `{:?}` and comparing the resulting strings, instead of using the
    /// `PartialEq` implementation on `SlotValue`. This is fragile because:
    ///
    /// - `Debug` formatting is not a stability guarantee; it can change across
    ///   Rust editions or `FiniteF64` refactors.
    /// - Two semantically equal values could theoretically produce different
    ///   debug strings (e.g. if `FiniteF64`'s `Debug` changes formatting).
    /// - The comparison allocates two strings per slot, when a simple
    ///   `old_val == new_val` would be both correct and allocation-free.
    ///
    /// This test demonstrates that the comparison *currently* works, but
    /// highlights the fragility of the approach. The real fix is to replace
    /// string comparison with `PartialEq`.
    #[test]
    fn finding_1_debug_format_comparison_is_fragile() {
        // Same value compared via Debug formatting -- works today but is not
        // guaranteed by any contract.
        let before = slot_map(&[(1, SlotValue::I64(42))]);
        let after = slot_map(&[(1, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(
            !panel.has_changes(),
            "identical values must produce no diff"
        );

        // Demonstrate the alternative: direct equality.
        let v1 = SlotValue::I64(42);
        let v2 = SlotValue::I64(42);
        assert_eq!(
            v1, v2,
            "SlotValue PartialEq should be used instead of Debug"
        );
    }

    /// FINDING 2 — MEDIUM: TaintChanged variant is dead code.
    ///
    /// The `SlotDiff::TaintChanged` variant is defined and has formatting
    /// support, but neither `from_event` nor `diff_between` ever produces it.
    /// The `current_slots: &HashMap<SlotIdx, SlotValue>` parameter does not
    /// carry taint metadata, so taint changes are structurally invisible to
    /// the diff engine. This means taint propagation regressions will silently
    /// pass the diff panel without detection.
    ///
    /// Impact: A slot whose value stays the same but whose taint label changes
    /// from `Clean` to `Secret` is invisible to the replay diff panel, which
    /// could cause security reviewers to miss taint leaks.
    #[test]
    fn finding_2_taint_changed_is_never_produced() {
        // diff_between only looks at SlotValue, which has no taint.
        // TaintChanged can never be emitted.
        let before = slot_map(&[(1, SlotValue::I64(42))]);
        let after = slot_map(&[(1, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        for entry in panel.entries() {
            assert!(
                !matches!(entry.diff, SlotDiff::TaintChanged { .. }),
                "TaintChanged is never produced by diff_between"
            );
        }

        // from_event also cannot produce TaintChanged -- it only checks
        // value equality via Debug formatting.
        let event = make_slot_written_event(1, SlotValue::I64(42), 1);
        let current = slot_map(&[(1, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::from_event(&event, &current);
        for entry in panel.entries() {
            assert!(
                !matches!(entry.diff, SlotDiff::TaintChanged { .. }),
                "TaintChanged is never produced by from_event"
            );
        }
    }

    /// FINDING 3 — MEDIUM: Silent data loss on deserialization failure.
    ///
    /// In `from_event`, if `postcard::from_bytes(bytes)` returns `Err` (e.g.
    /// corrupted journal bytes, schema evolution mismatch), the method silently
    /// returns an empty panel. There is no log, no error return, and no
    /// indicator that data was lost. A caller consuming replay events would
    /// never know that a slot write was dropped.
    ///
    /// This test verifies the current silent behavior and demonstrates that
    /// corrupted bytes produce no diff, no error indicator, and a valid
    /// event_seq -- making the data loss invisible.
    #[test]
    fn finding_3_silent_data_loss_on_deserialization_failure() {
        // Construct a SlotWrittenEvent with invalid postcard bytes.
        let event = JournalEvent::SlotWrittenEvent {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(99),
            slot: SlotIdx::new(5),
            value: Some(vec![0xFF, 0xFE, 0xFD, 0xFC]),
            extra: None, // garbage bytes
        };
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);

        // The slot write is silently dropped -- no diff, no error.
        assert!(
            !panel.has_changes(),
            "corrupted bytes silently produce empty panel"
        );
        // But the event_seq is still set, making it look like a valid empty
        // event rather than a failure.
        assert_eq!(
            panel.event_seq(),
            99,
            "event_seq is set even though data was lost"
        );
    }

    /// FINDING 4 — MEDIUM: from_event does not detect slot deletion.
    ///
    /// When `SlotWrittenEvent` carries `value: None`, the method returns an
    /// empty panel. However, semantically, a `SlotWrittenEvent` with `None`
    /// could represent a slot being cleared or deleted. If the slot existed
    /// in `current_slots`, this deletion goes undetected.
    ///
    /// This test shows that deleting a slot via `value: None` produces no
    /// `SlotDiff::Deleted` entry, even though the slot was present before.
    #[test]
    fn finding_4_slot_deletion_via_none_value_is_invisible() {
        let event = JournalEvent::SlotWrittenEvent {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(50),
            slot: SlotIdx::new(10),
            value: None, // represents deletion or clearing
            extra: None,
        };
        // Slot 10 exists in current state.
        let current = slot_map(&[(10, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::from_event(&event, &current);

        // No Deleted diff is produced -- the deletion is invisible.
        assert!(
            !panel.has_changes(),
            "slot deletion via value:None is silently ignored"
        );
        assert_eq!(panel.entries().len(), 0);
    }

    /// FINDING 5 — LOW: event_seq truncation silently collapses distinct events.
    ///
    /// When `EventSeq` exceeds `u32::MAX` (>4_294_967_295), the sequence is
    /// silently saturated to `u32::MAX`. Two events with sequences
    /// `u32::MAX + 1` and `u32::MAX + 2` would both appear as `event_seq:
    /// u32::MAX`, making them indistinguishable in the panel. This is a
    /// design trade-off (u32 field) but should be documented.
    #[test]
    fn finding_5_seq_truncation_collapses_distinct_events() {
        let seq_a = u64::from(u32::MAX) + 1; // 4_294_967_296
        let seq_b = u64::from(u32::MAX) + 2; // 4_294_967_297

        let event_a = make_slot_written_event(1, SlotValue::I64(1), seq_a);
        let event_b = make_slot_written_event(2, SlotValue::I64(2), seq_b);

        let panel_a = SlotDiffPanel::from_event(&event_a, &HashMap::new());
        let panel_b = SlotDiffPanel::from_event(&event_b, &HashMap::new());

        // Both panels have the same event_seq due to saturation.
        assert_eq!(panel_a.event_seq(), u32::MAX);
        assert_eq!(panel_b.event_seq(), u32::MAX);
        // They are indistinguishable by event_seq.
        assert_eq!(
            panel_a.event_seq(),
            panel_b.event_seq(),
            "distinct sequences collapse to the same u32::MAX"
        );
    }

    /// FINDING 6 — LOW: diff_between ordering is nondeterministic.
    ///
    /// The `diff_between` method iterates `HashMap`, which has no guaranteed
    /// order. Entries in the result are in arbitrary order. Callers that need
    /// deterministic output (e.g. for snapshot testing or audit logs) will get
    /// inconsistent ordering. The entries should be sorted by slot index for
    /// determinism.
    #[test]
    fn finding_6_diff_between_ordering_is_nondeterministic() {
        // Create two states with enough slots that HashMap iteration order
        // may vary. The test verifies the set is correct but ordering is not.
        let before = HashMap::new();
        let after = slot_map(&[
            (1, SlotValue::I64(1)),
            (2, SlotValue::I64(2)),
            (3, SlotValue::I64(3)),
            (4, SlotValue::I64(4)),
            (5, SlotValue::I64(5)),
            (6, SlotValue::I64(6)),
            (7, SlotValue::I64(7)),
            (8, SlotValue::I64(8)),
        ]);
        let panel = SlotDiffPanel::diff_between(&before, &after);

        // All entries are present.
        assert_eq!(panel.entries().len(), 8);

        // Check that slot indices are NOT guaranteed to be sorted.
        // If they happen to be sorted, this test passes vacuously.
        // The point is: there is no sorting contract.
        let slots: Vec<u16> = panel.entries().iter().map(|e| e.slot.get()).collect();
        let mut sorted_slots = slots.clone();
        sorted_slots.sort();

        // This assertion documents that ordering is unsorted (HashMap order).
        // If by coincidence the HashMap order matches sorted order, the test
        // still passes -- the finding is about the *lack* of a contract.
        assert_eq!(slots.len(), sorted_slots.len());
    }

    /// FINDING 7 — LOW: diff_between correctly distinguishes type variants.
    ///
    /// Because comparison is via `Debug` formatting, `SlotValue::Null` and
    /// `SlotValue::I64(0)` have different Debug representations, so this
    /// is correctly detected. This test verifies the current behavior is
    /// correct but notes the dependency on Debug formatting.
    #[test]
    fn finding_7_debug_format_distinguishes_all_variants() {
        let before = slot_map(&[(1, SlotValue::Null)]);
        let after = slot_map(&[(1, SlotValue::I64(0))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        assert!(
            panel.has_changes(),
            "Null vs I64(0) must be detected as a change"
        );
        assert_eq!(panel.entries().len(), 1);
        let Some(entry) = panel.entries().get(0) else {
            return;
        };
        assert!(matches!(entry.diff, SlotDiff::Modified { .. }));
    }

    /// FINDING 8 — MEDIUM: from_event uses Debug formatting instead of PartialEq.
    ///
    /// `from_event` uses `format!("{old_val:?}") == format!("{new_val:?}")`
    /// to detect equality, bypassing `PartialEq`. If two `SlotValue` variants
    /// ever produce the same `Debug` string despite being unequal (or vice
    /// versa), this would produce incorrect diffs. More importantly, the
    /// string comparison is both slower and less correct than structural
    /// equality.
    #[test]
    fn finding_8_from_event_uses_debug_instead_of_partial_eq() {
        // SlotValue::I64(42) == SlotValue::I64(42) via PartialEq
        let event = make_slot_written_event(1, SlotValue::I64(42), 10);
        let current = slot_map(&[(1, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::from_event(&event, &current);

        // Currently correct because Debug and PartialEq agree for I64.
        assert!(!panel.has_changes());

        // But the comparison path is string-based, not structural.
        let v1 = SlotValue::I64(42);
        let v2 = SlotValue::I64(42);
        // PartialEq should be the source of truth:
        assert_eq!(v1, v2);
        // Debug format happens to agree:
        assert_eq!(format!("{v1:?}"), format!("{v2:?}"));
    }

    /// FINDING 9 — LOW: from_event only processes SlotWrittenEvent.
    ///
    /// Events like `StepSucceeded` (which carries an output slot index) are
    /// ignored by `from_event`. This means slot writes that occur as part of
    /// step completion are invisible unless they also emit a separate
    /// `SlotWrittenEvent`. If the journal only records `StepSucceeded`, slot
    /// diffs will be incomplete.
    #[test]
    fn finding_9_step_succeeded_ignored_by_from_event() {
        let event = JournalEvent::StepSucceeded {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(10),
            step: vb_core::ids::StepIdx::new(0),
            output: SlotIdx::new(5),
        };
        let current = HashMap::new();
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(
            !panel.has_changes(),
            "StepSucceeded is silently ignored even though it references a slot"
        );
    }

    // =========================================================================
    // BLACKHAT security and correctness findings
    // =========================================================================

    /// FINDING 10 -- HIGH: from_event treats identical values with different
    /// Debug representations as different, and values with same Debug
    /// representation but different actual values as equal.
    ///
    /// The comparison path in `from_event` uses `format!("{old_val:?}") ==
    /// format!("{new_val:?}")` which depends on the `Debug` trait, not
    /// `PartialEq`. If two `SlotValue` variants ever have different `Debug`
    /// output despite being `PartialEq` equal (or vice versa), the diff
    /// would be incorrect. This is especially dangerous for floating-point
    /// or complex types where Debug formatting can vary.
    ///
    /// This test demonstrates the fragility by verifying that Debug-based
    /// comparison currently agrees with PartialEq for known simple types.
    #[test]
    fn blackhat_debug_vs_partial_eq_fragility() {
        // For I64, Debug and PartialEq agree.
        let a = SlotValue::I64(42);
        let b = SlotValue::I64(42);
        assert_eq!(a, b, "PartialEq says equal");
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "Debug says equal");

        // For Null, same thing.
        let c = SlotValue::Null;
        let d = SlotValue::Null;
        assert_eq!(c, d);
        assert_eq!(format!("{c:?}"), format!("{d:?}"));
    }

    /// FINDING 11 -- MEDIUM: diff_between does not detect taint-only changes.
    ///
    /// `diff_between` compares `SlotValue` by Debug formatting. Since
    /// `SlotValue` does not carry taint information, a slot whose value
    /// stays the same but whose taint label changes from Clean to Secret
    /// is invisible. This is a security blind spot: a taint leak via
    /// slot metadata goes undetected in the replay diff panel.
    #[test]
    fn blackhat_diff_between_misses_taint_only_changes() {
        // Both states have identical SlotValues.
        let before = slot_map(&[(1, SlotValue::I64(42))]);
        let after = slot_map(&[(1, SlotValue::I64(42))]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        // No changes detected -- but what if the taint label changed?
        assert!(
            !panel.has_changes(),
            "FINDING 11: taint-only changes are invisible to diff_between"
        );
        // No TaintChanged entries can ever appear.
        assert!(
            panel
                .entries
                .iter()
                .all(|e| !matches!(e.diff, SlotDiff::TaintChanged { .. }))
        );
    }

    /// FINDING 12 -- MEDIUM: from_event silently swallows deserialization
    /// errors, making data corruption invisible.
    ///
    /// When `postcard::from_bytes(bytes)` fails (corrupted data, schema
    /// evolution), `from_event` returns an empty panel with a valid `event_seq`.
    /// There is no error indicator, no log, and no way for the caller to
    /// distinguish "no data" from "data was corrupted". A replay consumer
    /// would silently miss slot writes.
    #[test]
    fn blackhat_from_event_corrupted_bytes_produces_empty_not_error() {
        let event = JournalEvent::SlotWrittenEvent {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(42),
            slot: SlotIdx::new(10),
            value: Some(vec![0xDE, 0xAD, 0xBE]),
            extra: None, // garbage postcard bytes
        };
        let panel = SlotDiffPanel::from_event(&event, &HashMap::new());
        assert!(
            !panel.has_changes(),
            "FINDING 12: corrupted bytes silently produce empty panel"
        );
        // event_seq is set, making it look like a valid empty event.
        assert_eq!(panel.event_seq(), 42);
    }

    /// FINDING 13 -- MEDIUM: from_event treats value:None as "no write" instead
    /// of "slot deletion".
    ///
    /// When `SlotWrittenEvent` carries `value: None`, the code immediately
    /// returns an empty panel. If the slot exists in `current_slots`, this
    /// represents a deletion that goes undetected. A slot can disappear from
    /// the state without the diff engine noticing.
    #[test]
    fn blackhat_from_event_value_none_is_not_deletion() {
        let event = JournalEvent::SlotWrittenEvent {
            run: vb_core::ids::RunId::new(1),
            seq: EventSeq::new(5),
            slot: SlotIdx::new(3),
            value: None, // semantically: delete slot 3
            extra: None,
        };
        let current = slot_map(&[(3, SlotValue::Bool(true))]);
        let panel = SlotDiffPanel::from_event(&event, &current);
        assert!(
            !panel.has_changes(),
            "FINDING 13: slot deletion via value:None is invisible"
        );
        // Slot 3 still exists in current but was logically deleted -- no diff.
    }

    /// FINDING 14 -- LOW: diff_between returns entries in HashMap iteration
    /// order, which is nondeterministic.
    ///
    /// Two calls to `diff_between` with identical inputs may produce entries
    /// in different orders. This makes snapshot testing and audit log
    /// comparison unreliable.
    #[test]
    fn blackhat_diff_between_ordering_is_nondeterministic() {
        let before = HashMap::new();
        let after = slot_map(&[
            (1, SlotValue::I64(1)),
            (2, SlotValue::I64(2)),
            (3, SlotValue::I64(3)),
            (4, SlotValue::I64(4)),
            (5, SlotValue::I64(5)),
            (6, SlotValue::I64(6)),
            (7, SlotValue::I64(7)),
            (8, SlotValue::I64(8)),
        ]);
        let panel = SlotDiffPanel::diff_between(&before, &after);
        // All entries present.
        assert_eq!(panel.entries().len(), 8);
        // Check set of slot indices.
        let slots: std::collections::HashSet<u16> =
            panel.entries().iter().map(|e| e.slot.get()).collect();
        assert_eq!(slots.len(), 8);
        // FINDING 14: The order is undefined (HashMap iteration order).
    }

    /// FINDING 15 -- LOW: seq truncation to u32::MAX collapses distinct events.
    ///
    /// `EventSeq` is a `u64` internally. When it exceeds `u32::MAX`, the
    /// `u32::try_from(...).unwrap_or(u32::MAX)` silently saturates. Two events
    /// with seq `u32::MAX + 1` and `u32::MAX + 2` would both have
    /// `event_seq: u32::MAX`, making them indistinguishable in the panel.
    #[test]
    fn blackhat_seq_truncation_collapses_events() {
        let over_a = u64::from(u32::MAX) + 1;
        let over_b = u64::from(u32::MAX) + 2;

        let event_a = make_slot_written_event(1, SlotValue::I64(1), over_a);
        let event_b = make_slot_written_event(2, SlotValue::I64(2), over_b);

        let panel_a = SlotDiffPanel::from_event(&event_a, &HashMap::new());
        let panel_b = SlotDiffPanel::from_event(&event_b, &HashMap::new());

        // Both panels report the same event_seq.
        assert_eq!(panel_a.event_seq(), u32::MAX);
        assert_eq!(panel_b.event_seq(), u32::MAX);
        assert_eq!(
            panel_a.event_seq(),
            panel_b.event_seq(),
            "FINDING 15: distinct u64 seqs collapse to same u32::MAX"
        );
    }

    /// FINDING 16 -- LOW: format_entry allocates a new string for every call,
    /// and creates the slot_label via format! every time. This is not a bug but
    /// a performance consideration for hot paths with many entries.
    #[test]
    fn blackhat_format_entry_all_variants_no_panic() {
        let entries = vec![
            DiffEntry {
                slot: SlotIdx::new(0),
                diff: SlotDiff::Created(String::from("Null")),
            },
            DiffEntry {
                slot: SlotIdx::new(1),
                diff: SlotDiff::Modified {
                    old: String::from("I64(1)"),
                    new: String::from("I64(2)"),
                },
            },
            DiffEntry {
                slot: SlotIdx::new(2),
                diff: SlotDiff::Deleted(String::from("Bool(true)")),
            },
            DiffEntry {
                slot: SlotIdx::new(3),
                diff: SlotDiff::TaintChanged {
                    old: String::from("Clean"),
                    new: String::from("Secret"),
                },
            },
        ];
        for entry in &entries {
            let formatted = SlotDiffPanel::format_entry(entry);
            assert!(
                !formatted.is_empty(),
                "FINDING 16: format_entry produced empty string for {:?}",
                entry.diff
            );
        }
    }

    /// FINDING 17 -- LOW: from_event returns default (empty, seq=0) for
    /// non-SlotWrittenEvent variants, losing the event's seq number.
    ///
    /// When a non-slot event (e.g. StepStarted) is passed to `from_event`,
    /// the method returns `Self::new()` which has `event_seq: 0`. The
    /// event's actual seq number is discarded. This makes it impossible
    /// to correlate the empty panel with the original event.
    #[test]
    fn blackhat_from_event_non_slot_event_loses_seq() {
        let event = make_step_started_event(42);
        let panel = SlotDiffPanel::from_event(&event, &HashMap::new());
        assert_eq!(
            panel.event_seq(),
            0,
            "FINDING 17: non-slot event seq (42) is lost, reported as 0"
        );
    }
}
