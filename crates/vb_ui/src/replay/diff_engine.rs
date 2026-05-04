//! Diff computation engine for replay comparison.
//!
//! Compares consecutive [`ReplaySnapshot`] pairs carried by [`ReplayEvent`]s,
//! producing structured [`StepDiff`] results that classify each slot change
//! as Added, Removed, Modified, or Unchanged with cyberpunk-palette colors.

use super::types::{ReplayEvent, ReplaySnapshot};

// ---------------------------------------------------------------------------
// Color constants (match types.rs palette)
// ---------------------------------------------------------------------------

/// Neon green (#39ff14) -- Added / Unchanged.
const NEON_GREEN: [f32; 4] = [0.224, 1.0, 0.078, 1.0];
/// Neon red (#ff073a) -- Removed.
const NEON_RED: [f32; 4] = [1.0, 0.027, 0.227, 1.0];
/// Neon cyan (#00f5ff) -- Modified.
const NEON_CYAN: [f32; 4] = [0.0, 0.961, 1.0, 1.0];
/// Text dim (#555577) -- Unchanged.
const TEXT_DIM: [f32; 4] = [0.333, 0.333, 0.467, 1.0];

// ---------------------------------------------------------------------------
// ChangeType
// ---------------------------------------------------------------------------

/// Classification of a slot change between two snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeType {
    /// Slot appeared in the after-snapshot (absent before).
    Added,
    /// Slot disappeared from the after-snapshot (present before).
    Removed,
    /// Slot value changed between snapshots.
    Modified,
    /// Slot value is identical in both snapshots.
    Unchanged,
}

impl ChangeType {
    /// Returns the cyberpunk palette RGBA color for this change type.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        match self {
            Self::Added => NEON_GREEN,
            Self::Removed => NEON_RED,
            Self::Modified => NEON_CYAN,
            Self::Unchanged => TEXT_DIM,
        }
    }
}

// ---------------------------------------------------------------------------
// SlotChange
// ---------------------------------------------------------------------------

/// Describes what happened to a single slot between two snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotChange {
    /// Slot identifier.
    pub slot: u32,
    /// Raw bytes before the change (empty if slot was absent).
    pub before: Vec<u8>,
    /// Raw bytes after the change (empty if slot was removed).
    pub after: Vec<u8>,
    /// Classification of the change.
    pub change_type: ChangeType,
    /// Render color derived from [`ChangeType::color`].
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// TaintDelta
// ---------------------------------------------------------------------------

/// Describes a taint state change for a slot between two snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct TaintDelta {
    /// Slot whose taint changed.
    pub slot: u32,
    /// Human-readable description of the kind change.
    pub kind_change: String,
    /// Render color for this delta.
    pub color: [f32; 4],
}

// ---------------------------------------------------------------------------
// StepDiff
// ---------------------------------------------------------------------------

/// Aggregate diff result for a single step transition.
#[derive(Debug, Clone, PartialEq)]
pub struct StepDiff {
    /// The step index this diff corresponds to.
    pub step: u16,
    /// Per-slot changes detected between the two snapshots.
    pub changes: Vec<SlotChange>,
    /// Taint deltas detected between the two snapshots.
    pub taint_deltas: Vec<TaintDelta>,
}

impl StepDiff {
    /// Returns `true` when at least one non-unchanged slot change or taint
    /// delta exists.
    #[must_use]
    pub fn has_changes(&self) -> bool {
        self.changes
            .iter()
            .any(|c| c.change_type != ChangeType::Unchanged)
            || !self.taint_deltas.is_empty()
    }

    /// Returns the count of non-unchanged slot changes plus taint deltas.
    #[must_use]
    pub fn change_count(&self) -> usize {
        let slot_count = self
            .changes
            .iter()
            .filter(|c| c.change_type != ChangeType::Unchanged)
            .count();
        slot_count.saturating_add(self.taint_deltas.len())
    }
}

// ---------------------------------------------------------------------------
// ReplayDiffEngine
// ---------------------------------------------------------------------------

/// Stateless diff computation engine for replay snapshot comparison.
///
/// All methods are pure functions over their inputs; the engine holds no
/// mutable state and is safe to share or reuse.
pub struct ReplayDiffEngine;

impl ReplayDiffEngine {
    /// Creates a new (stateless) diff engine.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Compares two snapshots and produces a [`StepDiff`].
    ///
    /// Slot values are compared byte-for-byte. The `step` field of the
    /// returned diff is taken from `after.step_index`.
    ///
    /// # Slot classification
    ///
    /// - **Added**: present in `after`, absent in `before`.
    /// - **Removed**: present in `before`, absent in `after`.
    /// - **Modified**: present in both, bytes differ.
    /// - **Unchanged**: present in both, bytes identical.
    ///
    /// Taint deltas are computed by comparing the raw `taint_state` byte
    /// vectors. If they differ, a single [`TaintDelta`] is emitted
    /// describing the change.
    #[must_use]
    pub fn diff_snapshots(&self, before: &ReplaySnapshot, after: &ReplaySnapshot) -> StepDiff {
        let changes = compute_slot_changes(&before.slot_values, &after.slot_values);
        let taint_deltas = compute_taint_deltas(&before.taint_state, &after.taint_state);

        StepDiff {
            step: after.step_index,
            changes,
            taint_deltas,
        }
    }

    /// Computes diffs for consecutive snapshot-bearing events.
    ///
    /// Iterates over `events`, pairing each event that carries a snapshot
    /// with the previous snapshot-bearing event, and calls
    /// [`Self::diff_snapshots`] on the pair. Events without snapshots are
    /// skipped.
    ///
    /// If fewer than two snapshot-bearing events exist, returns an empty
    /// vector.
    #[must_use]
    pub fn diff_events(&self, events: &[ReplayEvent]) -> Vec<StepDiff> {
        let mut results = Vec::new();
        let mut prev: Option<&ReplaySnapshot> = None;

        for event in events {
            let Some(ref snapshot) = event.snapshot else {
                continue;
            };

            if let Some(before) = prev {
                results.push(self.diff_snapshots(before, snapshot));
            }
            prev = Some(snapshot);
        }

        results
    }
}

impl Default for ReplayDiffEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Builds a sorted, deduplicated list of all slot ids present in either
/// snapshot's slot_values.
fn collect_all_slot_ids(before: &[(u32, Vec<u8>)], after: &[(u32, Vec<u8>)]) -> Vec<u32> {
    let mut ids = Vec::new();

    for &(slot, _) in before {
        ids.push(slot);
    }
    for &(slot, _) in after {
        ids.push(slot);
    }

    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Looks up a slot's bytes in a sorted-by-insertion slot_values list.
fn find_slot_bytes(slot_values: &[(u32, Vec<u8>)], target: u32) -> Option<&[u8]> {
    slot_values
        .iter()
        .find(|&&(slot, _)| slot == target)
        .map(|(_, bytes)| bytes.as_slice())
}

/// Classifies each slot across two snapshots and produces a `SlotChange`
/// for every slot present in either snapshot.
fn compute_slot_changes(
    before: &[(u32, Vec<u8>)],
    after: &[(u32, Vec<u8>)],
) -> Vec<SlotChange> {
    let all_ids = collect_all_slot_ids(before, after);
    let mut changes = Vec::with_capacity(all_ids.len());

    for slot in all_ids {
        let before_bytes = find_slot_bytes(before, slot);
        let after_bytes = find_slot_bytes(after, slot);

        let (change_type, before_vec, after_vec) = match (before_bytes, after_bytes) {
            (None, Some(a)) => (
                ChangeType::Added,
                Vec::new(),
                a.to_vec(),
            ),
            (Some(_), None) => (
                ChangeType::Removed,
                before_bytes
                    .map(|b| b.to_vec())
                    .unwrap_or_default(),
                Vec::new(),
            ),
            (Some(b), Some(a)) => {
                if b == a {
                    (
                        ChangeType::Unchanged,
                        b.to_vec(),
                        a.to_vec(),
                    )
                } else {
                    (
                        ChangeType::Modified,
                        b.to_vec(),
                        a.to_vec(),
                    )
                }
            }
            (None, None) => continue,
        };

        changes.push(SlotChange {
            slot,
            before: before_vec,
            after: after_vec,
            color: change_type.color(),
            change_type,
        });
    }

    changes
}

/// Compares two taint_state byte vectors. If they differ, produces a single
/// [`TaintDelta`] describing the change.
fn compute_taint_deltas(before: &[u8], after: &[u8]) -> Vec<TaintDelta> {
    if before == after {
        return Vec::new();
    }

    // Use a sentinel slot of 0 for the global taint state change.
    // Individual slot-level taint tracking would require structured taint
    // data, which is serialized into the byte vector.
    let kind_change = format!(
        "taint_state changed ({} bytes -> {} bytes)",
        before.len(),
        after.len(),
    );

    vec![TaintDelta {
        slot: 0,
        kind_change,
        color: NEON_CYAN,
    }]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::types::{ReplayEventType, ReplayStepDetail, ReplayStepStatus};

    // -- Helpers --

    fn make_snapshot(step: u16, slots: Vec<(u32, Vec<u8>)>, taint: Vec<u8>) -> ReplaySnapshot {
        ReplaySnapshot {
            step_index: step,
            slot_values: slots,
            taint_state: taint,
        }
    }

    fn make_event_with_snapshot(snapshot: ReplaySnapshot) -> ReplayEvent {
        ReplayEvent::with_snapshot(ReplayEventType::StepCompleted, snapshot)
    }

    fn make_event_no_snapshot() -> ReplayEvent {
        ReplayEvent::new(ReplayEventType::StepStarted)
    }

    // -- ChangeType::color --

    #[test]
    fn change_type_added_color_is_neon_green() {
        assert_eq!(ChangeType::Added.color(), NEON_GREEN);
    }

    #[test]
    fn change_type_removed_color_is_neon_red() {
        assert_eq!(ChangeType::Removed.color(), NEON_RED);
    }

    #[test]
    fn change_type_modified_color_is_neon_cyan() {
        assert_eq!(ChangeType::Modified.color(), NEON_CYAN);
    }

    #[test]
    fn change_type_unchanged_color_is_text_dim() {
        assert_eq!(ChangeType::Unchanged.color(), TEXT_DIM);
    }

    // -- ReplayDiffEngine construction --

    #[test]
    fn engine_new_is_default() {
        let a = ReplayDiffEngine::new();
        let b = ReplayDiffEngine::default();
        // Stateless -- both are equivalent.
        let snap = make_snapshot(0, Vec::new(), Vec::new());
        let diff_a = a.diff_snapshots(&snap, &snap);
        let diff_b = b.diff_snapshots(&snap, &snap);
        assert_eq!(diff_a, diff_b);
    }

    // -- diff_snapshots: empty snapshots --

    #[test]
    fn diff_snapshots_empty_yields_no_changes() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), Vec::new());
        let after = make_snapshot(1, Vec::new(), Vec::new());
        let diff = engine.diff_snapshots(&before, &after);
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
        assert_eq!(diff.step, 1);
        assert!(diff.changes.is_empty());
        assert!(diff.taint_deltas.is_empty());
    }

    // -- diff_snapshots: slot added --

    #[test]
    fn diff_snapshots_detects_added_slot() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), Vec::new());
        let after = make_snapshot(1, vec![(42u32, vec![1u8, 2, 3])], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);
        assert_eq!(diff.changes.len(), 1);

        let Some(change) = diff.changes.first() else {
            // Already asserted len == 1 above, so this branch is unreachable.
            return;
        };
        assert_eq!(change.slot, 42);
        assert!(change.before.is_empty());
        assert_eq!(change.after, vec![1u8, 2, 3]);
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.color, NEON_GREEN);
    }

    // -- diff_snapshots: slot removed --

    #[test]
    fn diff_snapshots_detects_removed_slot() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, vec![(10u32, vec![0xFFu8])], Vec::new());
        let after = make_snapshot(1, Vec::new(), Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);

        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.slot, 10);
        assert_eq!(change.before, vec![0xFFu8]);
        assert!(change.after.is_empty());
        assert_eq!(change.change_type, ChangeType::Removed);
        assert_eq!(change.color, NEON_RED);
    }

    // -- diff_snapshots: slot modified --

    #[test]
    fn diff_snapshots_detects_modified_slot() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, vec![(5u32, vec![1u8])], Vec::new());
        let after = make_snapshot(1, vec![(5u32, vec![2u8])], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);

        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.slot, 5);
        assert_eq!(change.before, vec![1u8]);
        assert_eq!(change.after, vec![2u8]);
        assert_eq!(change.change_type, ChangeType::Modified);
        assert_eq!(change.color, NEON_CYAN);
    }

    // -- diff_snapshots: slot unchanged --

    #[test]
    fn diff_snapshots_unchanged_slot_not_counted() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, vec![(1u32, vec![7u8, 8])], Vec::new());
        let after = make_snapshot(1, vec![(1u32, vec![7u8, 8])], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        // Unchanged slots are still recorded in changes but do not count
        // toward has_changes or change_count.
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
        assert_eq!(diff.changes.len(), 1);

        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.change_type, ChangeType::Unchanged);
        assert_eq!(change.color, TEXT_DIM);
    }

    // -- diff_snapshots: taint delta --

    #[test]
    fn diff_snapshots_detects_taint_delta() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), Vec::new());
        let after = make_snapshot(1, Vec::new(), vec![1u8]);
        let diff = engine.diff_snapshots(&before, &after);

        assert!(diff.has_changes());
        assert_eq!(diff.taint_deltas.len(), 1);

        let Some(delta) = diff.taint_deltas.first() else {
            return;
        };
        assert_eq!(delta.slot, 0);
        assert!(delta.kind_change.contains("0 bytes -> 1 bytes"));
        assert_eq!(delta.color, NEON_CYAN);
    }

    // -- diff_snapshots: multiple slot changes --

    #[test]
    fn diff_snapshots_multiple_slots() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(
            0,
            vec![
                (1u32, vec![10u8]),
                (2u32, vec![20u8]),
                (3u32, vec![30u8]),
            ],
            Vec::new(),
        );
        let after = make_snapshot(
            1,
            vec![
                (1u32, vec![10u8]), // unchanged
                (2u32, vec![99u8]), // modified
                // slot 3 removed
                (4u32, vec![40u8]), // added
            ],
            Vec::new(),
        );
        let diff = engine.diff_snapshots(&before, &after);

        assert_eq!(diff.changes.len(), 4);
        assert_eq!(diff.change_count(), 3); // unchanged slot excluded

        let mut by_slot: std::collections::HashMap<u32, &SlotChange> =
            std::collections::HashMap::new();
        for change in &diff.changes {
            by_slot.insert(change.slot, change);
        }

        assert_eq!(by_slot[&1].change_type, ChangeType::Unchanged);
        assert_eq!(by_slot[&2].change_type, ChangeType::Modified);
        assert_eq!(by_slot[&3].change_type, ChangeType::Removed);
        assert_eq!(by_slot[&4].change_type, ChangeType::Added);
    }

    // -- diff_events: empty input --

    #[test]
    fn diff_events_empty_input() {
        let engine = ReplayDiffEngine::new();
        let diffs = engine.diff_events(&[]);
        assert!(diffs.is_empty());
    }

    // -- diff_events: single event with snapshot --

    #[test]
    fn diff_events_single_snapshot_no_pair() {
        let engine = ReplayDiffEngine::new();
        let snap = make_snapshot(0, Vec::new(), Vec::new());
        let events = vec![make_event_with_snapshot(snap)];
        let diffs = engine.diff_events(&events);
        assert!(diffs.is_empty());
    }

    // -- diff_events: two snapshot-bearing events --

    #[test]
    fn diff_events_two_snapshots_produces_one_diff() {
        let engine = ReplayDiffEngine::new();
        let snap_before = make_snapshot(0, vec![(1u32, vec![0u8])], Vec::new());
        let snap_after = make_snapshot(1, vec![(1u32, vec![1u8])], Vec::new());
        let events = vec![
            make_event_with_snapshot(snap_before),
            make_event_with_snapshot(snap_after),
        ];
        let diffs = engine.diff_events(&events);

        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].step, 1);
        assert_eq!(diffs[0].change_count(), 1);
    }

    // -- diff_events: interleaved non-snapshot events are skipped --

    #[test]
    fn diff_events_skips_non_snapshot_events() {
        let engine = ReplayDiffEngine::new();
        let snap_a = make_snapshot(0, Vec::new(), Vec::new());
        let snap_b = make_snapshot(1, vec![(1u32, vec![42u8])], Vec::new());
        let snap_c = make_snapshot(2, vec![(1u32, vec![99u8])], Vec::new());
        let events = vec![
            make_event_with_snapshot(snap_a),
            make_event_no_snapshot(),
            make_event_no_snapshot(),
            make_event_with_snapshot(snap_b),
            make_event_no_snapshot(),
            make_event_with_snapshot(snap_c),
        ];
        let diffs = engine.diff_events(&events);

        assert_eq!(diffs.len(), 2);
        // diff between snap_a and snap_b (added slot 1)
        assert_eq!(diffs[0].step, 1);
        assert!(diffs[0].has_changes());
        // diff between snap_b and snap_c (modified slot 1)
        assert_eq!(diffs[1].step, 2);
        assert!(diffs[1].has_changes());
    }

    // -- StepDiff equality --

    #[test]
    fn step_diff_equality() {
        let a = StepDiff {
            step: 3,
            changes: vec![SlotChange {
                slot: 1,
                before: vec![],
                after: vec![1u8],
                change_type: ChangeType::Added,
                color: NEON_GREEN,
            }],
            taint_deltas: Vec::new(),
        };
        let b = StepDiff {
            step: 3,
            changes: vec![SlotChange {
                slot: 1,
                before: vec![],
                after: vec![1u8],
                change_type: ChangeType::Added,
                color: NEON_GREEN,
            }],
            taint_deltas: Vec::new(),
        };
        assert_eq!(a, b);
    }

    // -- TaintDelta equality --

    #[test]
    fn taint_delta_equality() {
        let a = TaintDelta {
            slot: 5,
            kind_change: String::from("changed"),
            color: NEON_CYAN,
        };
        let b = TaintDelta {
            slot: 5,
            kind_change: String::from("changed"),
            color: NEON_CYAN,
        };
        assert_eq!(a, b);
    }

    // -- SlotChange equality --

    #[test]
    fn slot_change_inequality_different_type() {
        let a = SlotChange {
            slot: 1,
            before: vec![],
            after: vec![1u8],
            change_type: ChangeType::Added,
            color: NEON_GREEN,
        };
        let b = SlotChange {
            slot: 1,
            before: vec![],
            after: vec![1u8],
            change_type: ChangeType::Modified,
            color: NEON_CYAN,
        };
        assert_ne!(a, b);
    }

    // -- diff_snapshots: step index comes from after --

    #[test]
    fn diff_snapshots_step_from_after() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(7, Vec::new(), Vec::new());
        let after = make_snapshot(42, Vec::new(), Vec::new());
        let diff = engine.diff_snapshots(&before, &after);
        assert_eq!(diff.step, 42);
    }

    // -- diff_events: events with step detail but no snapshot are skipped --

    #[test]
    fn diff_events_skips_events_with_detail_only() {
        let engine = ReplayDiffEngine::new();
        let detail = ReplayStepDetail {
            step_index: 5,
            node_label: String::from("test"),
            duration_us: Some(100),
            status: ReplayStepStatus::Running,
        };
        let snap = make_snapshot(0, Vec::new(), Vec::new());
        let events = vec![
            ReplayEvent::with_step_detail(ReplayEventType::StepStarted, detail),
            make_event_with_snapshot(snap),
        ];
        let diffs = engine.diff_events(&events);
        assert!(diffs.is_empty()); // only one snapshot-bearing event => no pair
    }

    // -- has_changes with only taint delta --

    #[test]
    fn step_diff_has_changes_with_only_taint_delta() {
        let diff = StepDiff {
            step: 0,
            changes: vec![SlotChange {
                slot: 1,
                before: vec![5u8],
                after: vec![5u8],
                change_type: ChangeType::Unchanged,
                color: TEXT_DIM,
            }],
            taint_deltas: vec![TaintDelta {
                slot: 0,
                kind_change: String::from("changed"),
                color: NEON_CYAN,
            }],
        };
        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);
    }

    // -- diff_snapshots: identical snapshots have no changes --

    #[test]
    fn diff_snapshots_identical_snapshots() {
        let engine = ReplayDiffEngine::new();
        let snap = make_snapshot(5, vec![(1u32, vec![10u8, 20])], vec![0u8, 1]);
        let diff = engine.diff_snapshots(&snap, &snap);

        // Same step index (from after), unchanged slot, same taint.
        assert_eq!(diff.step, 5);
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
        // Unchanged slot is still recorded.
        assert_eq!(diff.changes.len(), 1);
        assert!(diff.taint_deltas.is_empty());
    }
}
