#![forbid(unsafe_code)]
//! Unit tests for `BoundedOutcomeIndex`.

use vb_core::ids::RunId;

use crate::shard::bounded_outcomes::{BoundedOutcomeIndex, DEFAULT_MAX_TERMINAL_OUTCOMES};
use crate::shard::types::TerminalOutcome;

#[test]
fn bounded_outcome_index_starts_empty() {
    let index = BoundedOutcomeIndex::with_capacity(4);
    assert_eq!(index.len(), 0);
    assert!(index.is_empty());
    assert_eq!(index.capacity(), 4);
    assert_eq!(index.overflows(), 0);
}

#[test]
fn bounded_outcome_index_records_and_reads_outcome() {
    let mut index = BoundedOutcomeIndex::with_capacity(4);
    let run = RunId::new(1);
    index.record(run, TerminalOutcome::Cancelled);
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(run), Some(TerminalOutcome::Cancelled));
}

#[test]
fn bounded_outcome_index_idempotent_record_replaces_outcome() {
    let mut index = BoundedOutcomeIndex::with_capacity(4);
    let run = RunId::new(2);
    index.record(run, TerminalOutcome::Cancelled);
    index.record(run, TerminalOutcome::Killed);
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(run), Some(TerminalOutcome::Killed));
    assert_eq!(index.overflows(), 0);
}

#[test]
fn bounded_outcome_index_evicts_oldest_at_capacity() {
    let mut index = BoundedOutcomeIndex::with_capacity(2);
    index.record(RunId::new(10), TerminalOutcome::Cancelled);
    index.record(RunId::new(11), TerminalOutcome::Completed);
    index.record(RunId::new(12), TerminalOutcome::Killed);
    assert_eq!(index.len(), 2);
    assert_eq!(index.get(RunId::new(10)), None);
    assert_eq!(index.get(RunId::new(11)), Some(TerminalOutcome::Completed));
    assert_eq!(index.get(RunId::new(12)), Some(TerminalOutcome::Killed));
    assert_eq!(index.overflows(), 0);
}

#[test]
fn bounded_outcome_index_remove_returns_bool() {
    let mut index = BoundedOutcomeIndex::with_capacity(2);
    let run = RunId::new(20);
    index.record(run, TerminalOutcome::Cancelled);
    assert_eq!(index.remove(run), true);
    assert_eq!(index.remove(run), false);
}

#[test]
fn bounded_outcome_index_force_record_overflows() {
    let mut index = BoundedOutcomeIndex::with_capacity(1);
    index.record(RunId::new(30), TerminalOutcome::Cancelled);
    index.force_record(RunId::new(31), TerminalOutcome::Killed);
    assert_eq!(index.overflows(), 1);
    assert_eq!(index.len(), 2);
}

#[test]
fn bounded_outcome_index_default_capacity_matches_default_constant() {
    let index = BoundedOutcomeIndex::default();
    assert_eq!(index.capacity(), DEFAULT_MAX_TERMINAL_OUTCOMES);
}

#[test]
fn bounded_outcome_index_clear_preserves_capacity() {
    let mut index = BoundedOutcomeIndex::with_capacity(8);
    index.record(RunId::new(40), TerminalOutcome::Cancelled);
    index.record(RunId::new(41), TerminalOutcome::Killed);
    index.clear();
    assert_eq!(index.len(), 0);
    assert_eq!(index.capacity(), 8);
}
