#![forbid(unsafe_code)]
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
#[non_exhaustive]
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
fn compute_slot_changes(before: &[(u32, Vec<u8>)], after: &[(u32, Vec<u8>)]) -> Vec<SlotChange> {
    let all_ids = collect_all_slot_ids(before, after);
    let mut changes = Vec::with_capacity(all_ids.len());

    for slot in all_ids {
        let before_bytes = find_slot_bytes(before, slot);
        let after_bytes = find_slot_bytes(after, slot);

        let (change_type, before_vec, after_vec) = match (before_bytes, after_bytes) {
            (None, Some(a)) => (ChangeType::Added, Vec::new(), a.to_vec()),
            (Some(_), None) => (
                ChangeType::Removed,
                before_bytes.map(|b| b.to_vec()).unwrap_or_default(),
                Vec::new(),
            ),
            (Some(b), Some(a)) => {
                if b == a {
                    (ChangeType::Unchanged, b.to_vec(), a.to_vec())
                } else {
                    (ChangeType::Modified, b.to_vec(), a.to_vec())
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
            vec![(1u32, vec![10u8]), (2u32, vec![20u8]), (3u32, vec![30u8])],
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

    // =========================================================================
    // NEW TESTS
    // =========================================================================

    // -- 1. Multi-step rollback diffs --
    // Simulate state going forward (slots added/modified) then rolling back
    // (slots removed/reverted) to verify the diff engine handles reverse
    // transitions correctly.

    #[test]
    fn multi_step_rollback_produces_removed_changes() {
        let engine = ReplayDiffEngine::new();

        // Forward: step 0 -> step 1 adds slot 10 with [0xAA].
        let snap_0 = make_snapshot(0, Vec::new(), Vec::new());
        let snap_1 = make_snapshot(1, vec![(10u32, vec![0xAAu8])], Vec::new());
        let diff_forward = engine.diff_snapshots(&snap_0, &snap_1);
        assert_eq!(diff_forward.changes.len(), 1);
        let Some(change_fwd) = diff_forward.changes.first() else {
            return;
        };
        assert_eq!(change_fwd.change_type, ChangeType::Added);
        assert_eq!(change_fwd.slot, 10);

        // Rollback: step 1 -> step 0 removes slot 10.
        let diff_back = engine.diff_snapshots(&snap_1, &snap_0);
        assert_eq!(diff_back.changes.len(), 1);
        let Some(change_back) = diff_back.changes.first() else {
            return;
        };
        assert_eq!(change_back.change_type, ChangeType::Removed);
        assert_eq!(change_back.slot, 10);
        assert_eq!(change_back.before, vec![0xAAu8]);
        assert!(change_back.after.is_empty());
    }

    #[test]
    fn multi_step_rollback_reverts_modified_slot() {
        let engine = ReplayDiffEngine::new();

        // Step 0: slot 5 = [1, 2, 3]
        let snap_0 = make_snapshot(0, vec![(5u32, vec![1u8, 2, 3])], Vec::new());
        // Step 1: slot 5 modified to [9, 9, 9]
        let snap_1 = make_snapshot(1, vec![(5u32, vec![9u8, 9, 9])], Vec::new());
        // Step 2: rollback, slot 5 back to [1, 2, 3]
        let snap_2 = make_snapshot(2, vec![(5u32, vec![1u8, 2, 3])], Vec::new());

        let diff_forward = engine.diff_snapshots(&snap_0, &snap_1);
        let Some(cf) = diff_forward.changes.first() else {
            return;
        };
        assert_eq!(cf.change_type, ChangeType::Modified);
        assert_eq!(cf.after, vec![9u8, 9, 9]);

        let diff_back = engine.diff_snapshots(&snap_1, &snap_2);
        let Some(cb) = diff_back.changes.first() else {
            return;
        };
        assert_eq!(cb.change_type, ChangeType::Modified);
        assert_eq!(cb.before, vec![9u8, 9, 9]);
        assert_eq!(cb.after, vec![1u8, 2, 3]);
    }

    #[test]
    fn multi_step_rollback_taint_state_reverts() {
        let engine = ReplayDiffEngine::new();

        let snap_0 = make_snapshot(0, Vec::new(), Vec::new());
        let snap_1 = make_snapshot(1, Vec::new(), vec![0x01u8, 0x02]);
        let snap_2 = make_snapshot(2, Vec::new(), Vec::new());

        let diff_forward = engine.diff_snapshots(&snap_0, &snap_1);
        assert_eq!(diff_forward.taint_deltas.len(), 1);

        let diff_back = engine.diff_snapshots(&snap_1, &snap_2);
        assert_eq!(diff_back.taint_deltas.len(), 1);
        let Some(delta) = diff_back.taint_deltas.first() else {
            return;
        };
        assert!(delta.kind_change.contains("2 bytes -> 0 bytes"));
    }

    // -- 2. Cross-slot taint-propagation diffs --
    // Verify that when taint_state changes while multiple slots are also
    // changing, the diff engine correctly reports both slot changes and
    // taint deltas together.

    #[test]
    fn cross_slot_taint_with_multiple_slot_changes() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(
            0,
            vec![(1u32, vec![0x10u8]), (2u32, vec![0x20u8])],
            vec![0x00u8],
        );
        let after = make_snapshot(
            1,
            vec![
                (1u32, vec![0x11u8]), // modified
                (2u32, vec![0x20u8]), // unchanged
                (3u32, vec![0x30u8]), // added
            ],
            vec![0x01u8, 0x02],
        );

        let diff = engine.diff_snapshots(&before, &after);
        assert!(diff.has_changes());
        assert_eq!(diff.changes.len(), 3);
        assert_eq!(diff.taint_deltas.len(), 1);

        // Total change_count = 2 non-unchanged slots + 1 taint delta = 3
        assert_eq!(diff.change_count(), 3);
    }

    #[test]
    fn cross_slot_taint_propagation_delta_description() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(0, vec![(10u32, vec![1u8])], vec![0u8, 0, 0]);
        let after = make_snapshot(1, vec![(10u32, vec![2u8])], vec![0u8, 1, 0]);

        let diff = engine.diff_snapshots(&before, &after);
        assert_eq!(diff.taint_deltas.len(), 1);
        let Some(delta) = diff.taint_deltas.first() else {
            return;
        };
        assert!(delta.kind_change.contains("3 bytes -> 3 bytes"));
        assert_eq!(delta.color, NEON_CYAN);
    }

    // -- 3. Slot-type-change diffs --
    // The diff engine compares raw bytes; when a slot's byte representation
    // changes length or content entirely, it should classify as Modified.

    #[test]
    fn slot_type_change_shorter_bytes() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(0, vec![(7u32, vec![1u8, 2, 3, 4])], Vec::new());
        let after = make_snapshot(1, vec![(7u32, vec![0xFFu8])], Vec::new());

        let diff = engine.diff_snapshots(&before, &after);
        assert_eq!(diff.changes.len(), 1);
        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.change_type, ChangeType::Modified);
        assert_eq!(change.before, vec![1u8, 2, 3, 4]);
        assert_eq!(change.after, vec![0xFFu8]);
        assert_eq!(change.color, NEON_CYAN);
    }

    #[test]
    fn slot_type_change_longer_bytes() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(0, vec![(3u32, vec![0u8])], Vec::new());
        let after = make_snapshot(1, vec![(3u32, vec![0u8, 0, 0, 0, 0, 0, 0, 0])], Vec::new());

        let diff = engine.diff_snapshots(&before, &after);
        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.change_type, ChangeType::Modified);
        assert_eq!(change.before.len(), 1);
        assert_eq!(change.after.len(), 8);
    }

    #[test]
    fn slot_type_change_empty_to_nonempty_is_modified_not_added() {
        // When a slot id is present in both before and after, it is Modified
        // or Unchanged -- never Added or Removed -- even if bytes are empty.
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(0, vec![(99u32, Vec::new())], Vec::new());
        let after = make_snapshot(1, vec![(99u32, vec![0xDEu8, 0xAD])], Vec::new());

        let diff = engine.diff_snapshots(&before, &after);
        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.change_type, ChangeType::Modified);
    }

    // -- 4. Empty diff between identical frames --

    #[test]
    fn identical_frames_empty_slots_no_taint_no_changes() {
        let engine = ReplayDiffEngine::new();

        let frame_a = make_snapshot(10, Vec::new(), Vec::new());
        let frame_b = make_snapshot(11, Vec::new(), Vec::new());

        let diff = engine.diff_snapshots(&frame_a, &frame_b);
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
        assert!(diff.changes.is_empty());
        assert!(diff.taint_deltas.is_empty());
        assert_eq!(diff.step, 11);
    }

    #[test]
    fn identical_frames_with_slots_and_taint_no_changes() {
        let engine = ReplayDiffEngine::new();

        let frame = make_snapshot(
            5,
            vec![
                (1u32, vec![10u8, 20]),
                (2u32, vec![30u8]),
                (3u32, vec![40u8, 50, 60]),
            ],
            vec![0u8, 1, 2],
        );
        // Same frame used as both before and after.
        let diff = engine.diff_snapshots(&frame, &frame);

        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
        assert_eq!(diff.changes.len(), 3); // slots are recorded as Unchanged
        assert!(diff.taint_deltas.is_empty());

        for change in &diff.changes {
            assert_eq!(change.change_type, ChangeType::Unchanged);
            assert_eq!(change.color, TEXT_DIM);
        }
    }

    // -- 5. Diff with only step-state changes (no slot changes) --
    // Step index changes but slot layout is identical.

    #[test]
    fn only_step_index_changes_no_slot_changes() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(3, vec![(1u32, vec![42u8])], Vec::new());
        let after = make_snapshot(99, vec![(1u32, vec![42u8])], Vec::new());

        let diff = engine.diff_snapshots(&before, &after);
        assert_eq!(diff.step, 99);
        assert!(!diff.has_changes());
        assert_eq!(diff.change_count(), 0);
        assert_eq!(diff.changes.len(), 1);
        assert!(diff.taint_deltas.is_empty());
    }

    #[test]
    fn only_taint_changes_no_slot_changes() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(0, vec![(5u32, vec![1u8])], vec![0u8]);
        let after = make_snapshot(1, vec![(5u32, vec![1u8])], vec![1u8]);

        let diff = engine.diff_snapshots(&before, &after);
        // Slot 5 is unchanged, but taint changed.
        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);
        assert_eq!(diff.changes.len(), 1);
        assert_eq!(diff.changes[0].change_type, ChangeType::Unchanged);
        assert_eq!(diff.taint_deltas.len(), 1);
    }

    // -- 6. Diff with only slot changes (no step-state changes) --
    // Step index is the same, taint is the same, but slot bytes change.

    #[test]
    fn only_slot_changes_same_step_same_taint() {
        let engine = ReplayDiffEngine::new();

        let before = make_snapshot(4, vec![(8u32, vec![0u8])], vec![0xAAu8]);
        let after = make_snapshot(4, vec![(8u32, vec![1u8])], vec![0xAAu8]);

        let diff = engine.diff_snapshots(&before, &after);
        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);
        assert!(diff.taint_deltas.is_empty());
        assert_eq!(diff.step, 4);

        let Some(change) = diff.changes.first() else {
            return;
        };
        assert_eq!(change.change_type, ChangeType::Modified);
    }

    #[test]
    fn only_slot_added_no_taint_change() {
        let engine = ReplayDiffEngine::new();

        let taint = vec![0xF0u8];
        let before = make_snapshot(0, Vec::new(), taint.clone());
        let after = make_snapshot(0, vec![(20u32, vec![7u8])], taint);

        let diff = engine.diff_snapshots(&before, &after);
        assert!(diff.has_changes());
        assert_eq!(diff.change_count(), 1);
        assert!(diff.taint_deltas.is_empty());
    }

    // -- 8. Consecutive diffs applied in sequence --

    #[test]
    fn consecutive_diffs_applied_in_sequence_via_diff_events() {
        let engine = ReplayDiffEngine::new();

        // Step 0: slot 1 = [0x01]
        let snap_0 = make_snapshot(0, vec![(1u32, vec![0x01u8])], Vec::new());
        // Step 1: slot 1 = [0x02], slot 2 = [0x03] added
        let snap_1 = make_snapshot(
            1,
            vec![(1u32, vec![0x02u8]), (2u32, vec![0x03u8])],
            vec![0x01u8],
        );
        // Step 2: slot 1 = [0x02] (unchanged), slot 2 removed, taint changes
        let snap_2 = make_snapshot(2, vec![(1u32, vec![0x02u8])], vec![0x02u8]);
        // Step 3: slot 1 removed
        let snap_3 = make_snapshot(3, Vec::new(), vec![0x02u8]);

        let events = vec![
            make_event_with_snapshot(snap_0),
            make_event_with_snapshot(snap_1),
            make_event_with_snapshot(snap_2),
            make_event_with_snapshot(snap_3),
        ];

        let diffs = engine.diff_events(&events);
        assert_eq!(diffs.len(), 3);

        // Diff 0->1: slot 1 modified, slot 2 added, taint added
        assert_eq!(diffs[0].step, 1);
        assert_eq!(diffs[0].changes.len(), 2);
        assert_eq!(diffs[0].taint_deltas.len(), 1);
        assert_eq!(diffs[0].change_count(), 3);

        // Diff 1->2: slot 1 unchanged, slot 2 removed, taint modified
        assert_eq!(diffs[1].step, 2);
        assert_eq!(diffs[1].changes.len(), 2);
        assert_eq!(diffs[1].taint_deltas.len(), 1);
        assert_eq!(diffs[1].change_count(), 2); // unchanged slot excluded

        // Diff 2->3: slot 1 removed, no taint change
        assert_eq!(diffs[2].step, 3);
        assert_eq!(diffs[2].changes.len(), 1);
        assert!(diffs[2].taint_deltas.is_empty());
        assert_eq!(diffs[2].change_count(), 1);
    }

    #[test]
    fn consecutive_diffs_chain_from_empty_to_full_state() {
        let engine = ReplayDiffEngine::new();

        // Start completely empty.
        let snap_0 = make_snapshot(0, Vec::new(), Vec::new());
        // Add 3 slots.
        let snap_1 = make_snapshot(
            1,
            vec![(10u32, vec![1u8]), (20u32, vec![2u8]), (30u32, vec![3u8])],
            vec![0u8],
        );
        // Modify all 3 slots.
        let snap_2 = make_snapshot(
            2,
            vec![
                (10u32, vec![0x10u8]),
                (20u32, vec![0x20u8]),
                (30u32, vec![0x30u8]),
            ],
            vec![0u8, 1],
        );
        // Remove all slots (back to empty).
        let snap_3 = make_snapshot(3, Vec::new(), vec![0u8, 1]);

        let diffs = engine.diff_events(&events_from_snapshots(&[snap_0, snap_1, snap_2, snap_3]));

        assert_eq!(diffs.len(), 3);

        // All 3 added in first transition.
        let added = diffs[0]
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Added)
            .count();
        assert_eq!(added, 3);

        // All 3 modified in second transition.
        let modified = diffs[1]
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Modified)
            .count();
        assert_eq!(modified, 3);

        // All 3 removed in third transition.
        let removed = diffs[2]
            .changes
            .iter()
            .filter(|c| c.change_type == ChangeType::Removed)
            .count();
        assert_eq!(removed, 3);
    }

    // -- Helper for consecutive diff test --

    fn events_from_snapshots(snaps: &[ReplaySnapshot]) -> Vec<ReplayEvent> {
        snaps
            .iter()
            .map(|s| make_event_with_snapshot(s.clone()))
            .collect()
    }

    // =========================================================================
    // BLACKHAT security and correctness findings
    // =========================================================================

    /// FINDING 1 -- HIGH: compute_taint_deltas uses a single sentinel slot=0
    /// for the entire taint state change, losing per-slot taint granularity.
    ///
    /// The function compares the raw `taint_state` byte vectors. If they differ,
    /// it emits a single `TaintDelta` with `slot: 0` (sentinel). This means:
    /// - If 3 slots change taint simultaneously, only ONE delta is emitted.
    /// - The sentinel slot=0 could collide with actual slot 0 taint changes.
    /// - The description only reports byte length changes, not WHICH slots
    ///   changed taint.
    ///
    /// Impact: Taint propagation from Secret to Clean on a specific slot is
    /// invisible at the per-slot level. Security reviewers cannot determine
    /// which specific slot had its taint changed.
    #[test]
    fn blackhat_taint_delta_single_sentinel_loses_per_slot_granularity() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), vec![0x00u8, 0x01, 0x00]);
        let after = make_snapshot(1, Vec::new(), vec![0x01u8, 0x01, 0x00]);
        let diff = engine.diff_snapshots(&before, &after);

        assert_eq!(diff.taint_deltas.len(), 1);
        let delta = diff.taint_deltas.first().expect("delta");
        assert_eq!(
            delta.slot, 0,
            "FINDING 1: taint delta uses sentinel slot=0, losing per-slot granularity"
        );
        assert!(
            delta.kind_change.contains("3 bytes -> 3 bytes"),
            "FINDING 1: description only shows byte lengths, not actual taint changes"
        );
    }

    /// FINDING 2 -- MEDIUM: taint_delta description is misleading when byte
    /// vectors have the same length but different content.
    ///
    /// When taint_state changes from [0x00, 0x01] to [0x01, 0x01], the
    /// description says "2 bytes -> 2 bytes" which looks like no change.
    /// A user reading "2 bytes -> 2 bytes" would reasonably assume nothing
    /// changed, but the taint state DID change.
    #[test]
    fn blackhat_taint_delta_description_misleading_same_length() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), vec![0x00u8, 0x01]);
        let after = make_snapshot(1, Vec::new(), vec![0x01u8, 0x01]);
        let diff = engine.diff_snapshots(&before, &after);

        assert_eq!(diff.taint_deltas.len(), 1);
        let delta = diff.taint_deltas.first().expect("delta");
        // The description says "2 bytes -> 2 bytes" -- misleading.
        assert_eq!(
            delta.kind_change, "taint_state changed (2 bytes -> 2 bytes)",
            "FINDING 2: same-length taint change has misleading description"
        );
        // The delta IS reported, but the message looks like no-op.
    }

    /// FINDING 3 -- MEDIUM: find_slot_bytes is O(n) linear scan, not binary
    /// search, making it inefficient for large slot sets.
    ///
    /// `collect_all_slot_ids` collects all slot IDs into a sorted, deduplicated
    /// list. Then for each slot, `find_slot_bytes` does a linear scan over the
    /// slot_values vec. If slot_values has N entries and M unique slots across
    /// both snapshots, the total cost is O(N*M) instead of O(M log N) with
    /// binary search. For large slot sets, this is a performance concern.
    ///
    /// More critically: if `slot_values` has duplicate slot IDs within a single
    /// snapshot, `find_slot_bytes` returns the FIRST match. This means
    /// duplicate slot IDs within a snapshot are silently collapsed.
    #[test]
    fn blackhat_duplicate_slot_ids_in_snapshot_first_match_wins() {
        let engine = ReplayDiffEngine::new();
        // Before: slot 5 has TWO entries -- which one is used?
        let before = make_snapshot(
            0,
            vec![
                (5u32, vec![1u8]),
                (5u32, vec![2u8]), // duplicate slot ID
            ],
            Vec::new(),
        );
        let after = make_snapshot(1, Vec::new(), Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        // collect_all_slot_ids dedups, so only one slot 5 appears.
        assert_eq!(diff.changes.len(), 1);
        let change = diff.changes.first().expect("change");
        assert_eq!(change.slot, 5);
        // find_slot_bytes returns the FIRST match, so before = [1], not [2].
        assert_eq!(
            change.before,
            vec![1u8],
            "FINDING 3: duplicate slot ID uses first match, second value is silently lost"
        );
    }

    /// FINDING 4 -- LOW: SlotChange stores both `change_type` and redundant
    /// `color` field which is always `change_type.color()`.
    ///
    /// Every `SlotChange` stores a `color` field that is derived exclusively
    /// from `change_type`. This is redundant and creates an invariant that
    /// must be manually maintained. If someone constructs a `SlotChange` with
    /// mismatched `change_type` and `color`, there is no validation.
    #[test]
    fn blackhat_slot_change_color_redundant_with_change_type() {
        // Verify the engine always sets color = change_type.color().
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), Vec::new());
        let after = make_snapshot(1, vec![(1u32, vec![1u8])], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        for change in &diff.changes {
            assert_eq!(
                change.color,
                change.change_type.color(),
                "FINDING 4: color must match change_type.color()"
            );
        }
    }

    /// FINDING 5 -- MEDIUM: change_count can overflow for very large changes.
    ///
    /// `change_count` uses `saturating_add(self.taint_deltas.len())` which is
    /// correct for overflow safety. However, the `changes.iter().filter().count()`
    /// result is NOT protected -- if `changes` has more than `usize::MAX - N`
    /// non-unchanged entries where N is `taint_deltas.len()`, the saturating_add
    /// correctly saturates at `usize::MAX`. This is actually correct behavior;
    /// the finding is that the implementation is sound but relies on saturating
    /// arithmetic as a safety net rather than preventing the condition.
    #[test]
    fn blackhat_change_count_saturating_add_is_sound() {
        let diff = StepDiff {
            step: 0,
            changes: vec![
                SlotChange {
                    slot: 1,
                    before: vec![],
                    after: vec![1u8],
                    change_type: ChangeType::Added,
                    color: NEON_GREEN,
                },
                SlotChange {
                    slot: 2,
                    before: vec![],
                    after: vec![2u8],
                    change_type: ChangeType::Added,
                    color: NEON_GREEN,
                },
            ],
            taint_deltas: vec![TaintDelta {
                slot: 0,
                kind_change: String::from("changed"),
                color: NEON_CYAN,
            }],
        };
        assert_eq!(diff.change_count(), 3);
    }

    /// FINDING 6 -- MEDIUM: diff_events pairs consecutive snapshots regardless
    /// of their step_index ordering.
    ///
    /// If events arrive out of order (step_index 5 before step_index 3), the
    /// diff engine will still pair them as before/after. The resulting StepDiff
    /// will have `step` from the `after` snapshot, but the diff direction is
    /// reversed. This can produce misleading Added/Removed classifications.
    #[test]
    fn blackhat_diff_events_pairs_consecutive_snapshots_not_ordered_by_step() {
        let engine = ReplayDiffEngine::new();
        // Out-of-order: step 5 arrives before step 3.
        let snap_5 = make_snapshot(5, vec![(1u32, vec![42u8])], Vec::new());
        let snap_3 = make_snapshot(3, Vec::new(), Vec::new());
        let events = vec![
            make_event_with_snapshot(snap_5),
            make_event_with_snapshot(snap_3),
        ];
        let diffs = engine.diff_events(&events);
        assert_eq!(diffs.len(), 1);
        // The diff reports step=3 (from "after"), but the logical order is
        // step 3 -> step 5 (slot added), not step 5 -> step 3 (slot removed).
        assert_eq!(diffs[0].step, 3);
        // The diff says the slot was Removed, but logically it was Added
        // between steps 3 and 5.
        assert_eq!(
            diffs[0].changes.first().map(|c| c.change_type),
            Some(ChangeType::Removed),
            "FINDING 6: out-of-order steps produce misleading Removed instead of Added"
        );
    }

    /// FINDING 7 -- LOW: compute_slot_changes allocates two Vecs per slot
    /// (before and after), even for Unchanged slots.
    ///
    /// For Unchanged slots, both `before` and `after` contain identical bytes,
    /// wasting memory. In a snapshot with many unchanged slots, this doubles
    /// memory usage.
    #[test]
    fn blackhat_unchanged_slots_allocate_duplicate_bytes() {
        let engine = ReplayDiffEngine::new();
        let large_bytes: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let before = make_snapshot(0, vec![(1u32, large_bytes.clone())], Vec::new());
        let after = make_snapshot(1, vec![(1u32, large_bytes.clone())], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);

        assert_eq!(diff.changes.len(), 1);
        let change = diff.changes.first().expect("change");
        assert_eq!(change.change_type, ChangeType::Unchanged);
        // Both before and after hold copies of the same 1000 bytes.
        assert_eq!(change.before.len(), 1000);
        assert_eq!(change.after.len(), 1000);
        assert_eq!(change.before, change.after);
        // FINDING 7: two 1000-byte allocations for an unchanged slot.
    }

    /// FINDING 8 -- LOW: compute_slot_changes uses `(None, None)` arm as
    /// `continue`, which is unreachable given how collect_all_slot_ids works.
    ///
    /// `collect_all_slot_ids` only collects IDs from before and after. For each
    /// ID, at least one of before/after must have bytes. The `(None, None)`
    /// match arm is dead code. This is not a bug but indicates a missing
    /// code path that could mask a logic error if `collect_all_slot_ids`
    /// were changed to produce IDs not present in either snapshot.
    #[test]
    fn blackhat_none_none_arm_is_dead_code() {
        let engine = ReplayDiffEngine::new();
        // Every ID in collect_all_slot_ids comes from before or after,
        // so (None, None) is unreachable.
        let before = make_snapshot(0, vec![(1u32, vec![1u8])], Vec::new());
        let after = make_snapshot(1, vec![(2u32, vec![2u8])], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);
        // Slot 1: Removed, Slot 2: Added. No (None, None) possible.
        assert_eq!(diff.changes.len(), 2);
        for change in &diff.changes {
            assert!(
                !change.before.is_empty() || !change.after.is_empty(),
                "FINDING 8: (None, None) arm is dead code"
            );
        }
    }

    /// FINDING 9 -- LOW: step index in StepDiff is u16, matching ReplaySnapshot.
    /// Verify boundary step indices work correctly.
    #[test]
    fn blackhat_step_index_boundary_values() {
        let engine = ReplayDiffEngine::new();
        let before = make_snapshot(0, Vec::new(), Vec::new());
        let after = make_snapshot(u16::MAX, vec![(1u32, vec![1u8])], Vec::new());
        let diff = engine.diff_snapshots(&before, &after);
        assert_eq!(
            diff.step,
            u16::MAX,
            "FINDING 9: step u16::MAX should work correctly"
        );
        assert_eq!(diff.change_count(), 1);
    }

    /// FINDING 10 -- MEDIUM: diff_events does not validate that event types
    /// are semantically compatible with their snapshots.
    ///
    /// An event with `ReplayEventType::SlotWritten` but no snapshot will be
    /// skipped, while an event with `ReplayEventType::StepStarted` but WITH
    /// a snapshot will be included in the diff chain. The engine does not
    /// validate that snapshot-bearing events are semantically appropriate.
    #[test]
    fn blackhat_diff_events_includes_mismatched_event_type_with_snapshot() {
        let engine = ReplayDiffEngine::new();
        let snap_a = make_snapshot(0, vec![(1u32, vec![0u8])], Vec::new());
        // StepStarted event carrying a snapshot -- semantically odd but not rejected.
        let event_mismatched = ReplayEvent::with_snapshot(
            ReplayEventType::StepStarted,
            make_snapshot(1, vec![(1u32, vec![1u8])], Vec::new()),
        );
        let events = vec![make_event_with_snapshot(snap_a), event_mismatched];
        let diffs = engine.diff_events(&events);
        // The mismatched event is included because it has a snapshot.
        assert_eq!(
            diffs.len(),
            1,
            "FINDING 10: mismatched event type with snapshot is included in diff chain"
        );
    }
}
