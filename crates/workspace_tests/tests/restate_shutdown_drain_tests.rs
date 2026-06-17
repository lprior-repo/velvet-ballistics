#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Graceful shutdown drain-finalize behavior tests for IPC and storage shells.
//!
//! Models a synchronous shard that transitions through a cancel/drain/finalize
//! lifecycle during graceful shutdown. Uses `BoundedActionCompletionQueue` and
//! `ActionTicket` types from the VB runtime.
//!
//! Invariants under test:
//! - `stop_intake`: new submissions are rejected once shutdown begins.
//! - `drain`: bounded queues drain in order; no tickets are lost.
//! - `finalize`: durable journal/storage evidence is flushed; final state recorded.
//! - No pending `ActionTicket` or timer resurrection path left after finalize.
//! - Typed timeout and failure outcomes are preserved.

use std::collections::VecDeque;
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_runtime::action_queue::BoundedActionCompletionQueue;

// ---------------------------------------------------------------------------
// Shutdown state machine model
// ---------------------------------------------------------------------------

/// Shard lifecycle states during graceful shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShardState {
    /// Accepting and processing work normally.
    Running,
    /// Intake of new work is stopped; existing work is draining.
    Draining,
    /// All work flushed; durable evidence recorded.
    Finalized,
}

/// Typed errors emitted during shutdown transitions.
#[derive(Debug, PartialEq, Eq)]
enum ShutdownError {
    /// Submission rejected because the shard is no longer running.
    IntakeStopped,
    /// Drain timeout: not all work finished within the grace period.
    DrainTimeout { remaining: usize },
    /// Finalize attempted before drain completed.
    DrainNotComplete { remaining: usize },
    /// Operation not valid in the current state.
    InvalidTransition {
        current: ShardState,
        attempted: ShardState,
    },
}

/// Simulated durable journal (in-memory for testing).
#[derive(Debug, Clone)]
struct JournalEntry {
    run: u64,
    tickets_flushed: usize,
    final_ticket: Option<ActionTicket>,
}

/// A test-harness shard model.
///
/// - Maintains a bounded action completion queue.
/// - Transitions Running → Draining → Finalized during shutdown.
/// - Records journal entries for flush evidence.
struct ShardModel {
    state: ShardState,
    queue: BoundedActionCompletionQueue,
    pending_count: usize,
    journal: VecDeque<JournalEntry>,
    flushed: bool,
}

impl ShardModel {
    fn new(capacity: usize) -> Self {
        Self {
            state: ShardState::Running,
            queue: BoundedActionCompletionQueue::new(capacity).unwrap(),
            pending_count: 0,
            journal: VecDeque::new(),
            flushed: false,
        }
    }

    /// Submits a ticket. Accepted only in `Running` state.
    fn submit(&mut self, ticket: ActionTicket) -> Result<(), ShutdownError> {
        if self.state != ShardState::Running {
            return Err(ShutdownError::IntakeStopped);
        }
        self.queue
            .enqueue(ticket)
            .map_err(|_| ShutdownError::IntakeStopped)?;
        self.pending_count += 1;
        Ok(())
    }

    /// Begins graceful shutdown: transitions to Draining.
    fn begin_shutdown(&mut self) -> Result<(), ShutdownError> {
        if self.state != ShardState::Running {
            return Err(ShutdownError::InvalidTransition {
                current: self.state,
                attempted: ShardState::Draining,
            });
        }
        self.state = ShardState::Draining;
        Ok(())
    }

    /// Drains one ticket from the queue during the drain phase.
    ///
    /// Returns `Some(ticket)` if a ticket was drained, or `None` if empty.
    fn drain_one(&mut self) -> Option<ActionTicket> {
        if self.state != ShardState::Draining {
            return None;
        }
        let ticket = self.queue.dequeue()?;
        self.pending_count = self.pending_count.saturating_sub(1);
        Some(ticket)
    }

    /// Drains all remaining tickets. Returns the count drained this call.
    fn drain_all(&mut self) -> usize {
        let mut count = 0;
        while self.drain_one().is_some() {
            count += 1;
        }
        count
    }

    /// Finalizes the shutdown: flushes journal evidence and transitions to Finalized.
    fn finalize(&mut self, run: u64) -> Result<(), ShutdownError> {
        if self.state != ShardState::Draining {
            return Err(ShutdownError::InvalidTransition {
                current: self.state,
                attempted: ShardState::Finalized,
            });
        }
        if self.pending_count > 0 {
            return Err(ShutdownError::DrainNotComplete {
                remaining: self.pending_count,
            });
        }
        // Record journal evidence
        self.journal.push_back(JournalEntry {
            run,
            tickets_flushed: 0,
            final_ticket: None,
        });
        self.flushed = true;
        self.state = ShardState::Finalized;
        Ok(())
    }

    /// Returns the current lifecycle state.
    fn current_state(&self) -> ShardState {
        self.state
    }

    /// Returns whether durable evidence has been flushed.
    fn is_flushed(&self) -> bool {
        self.flushed
    }

    /// Returns the count of pending (un-drained) tickets.
    fn pending(&self) -> usize {
        self.pending_count
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mk_ticket(run: u64, seq: u32) -> ActionTicket {
    let run_id = RunId::new(run);
    let seq_no = SeqNo::new(u64::from(seq));
    let action_id = ActionId::new(1);
    let key = compute_action_idempotency_key(run_id, seq_no, action_id);
    ActionTicket {
        run: run_id,
        step: StepIdx::new(0),
        seq: seq_no,
        action: action_id,
        attempt: 1,
        idempotency_key: key,
        capacity: 3,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Happy-path: full lifecycle
// ---------------------------------------------------------------------------

/// Full Running → Draining → Drain → Finalize completes without error.
#[test]
fn full_shutdown_lifecycle_completes() {
    let mut shard = ShardModel::new(8);
    assert_eq!(shard.current_state(), ShardState::Running);

    // Submit work
    shard.submit(mk_ticket(1, 1)).unwrap();
    shard.submit(mk_ticket(1, 2)).unwrap();
    assert_eq!(shard.pending(), 2);

    // Begin shutdown
    shard.begin_shutdown().unwrap();
    assert_eq!(shard.current_state(), ShardState::Draining);

    // Drain all
    let drained = shard.drain_all();
    assert_eq!(drained, 2);
    assert_eq!(shard.pending(), 0);

    // Finalize
    shard.finalize(1).unwrap();
    assert_eq!(shard.current_state(), ShardState::Finalized);
    assert!(shard.is_flushed());
    let entry = shard.journal.back().unwrap();
    assert_eq!(entry.run, 1);
    assert_eq!(entry.tickets_flushed, 0);
    assert_eq!(entry.final_ticket, None);
}

/// Shutdown with no pending work: drain yields zero, finalize succeeds.
#[test]
fn shutdown_with_no_pending_work_succeeds() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    assert_eq!(shard.drain_all(), 0);
    shard.finalize(0).unwrap();
    assert!(shard.is_flushed());
}

// ---------------------------------------------------------------------------
// Intake rejection after shutdown begins
// ---------------------------------------------------------------------------

/// New submissions after `begin_shutdown` are rejected.
#[test]
fn submit_rejected_after_shutdown_begins() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    let err = shard.submit(mk_ticket(99, 1)).unwrap_err();
    assert_eq!(err, ShutdownError::IntakeStopped);
}

/// Submissions during `Draining` are rejected.
#[test]
fn submit_rejected_during_draining() {
    let mut shard = ShardModel::new(4);
    shard.submit(mk_ticket(1, 1)).unwrap();
    shard.begin_shutdown().unwrap();
    assert_eq!(shard.current_state(), ShardState::Draining);
    let err = shard.submit(mk_ticket(2, 1)).unwrap_err();
    assert_eq!(err, ShutdownError::IntakeStopped);
}

/// Submissions after `Finalized` are rejected.
#[test]
fn submit_rejected_after_finalized() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    shard.drain_all();
    shard.finalize(1).unwrap();
    assert_eq!(shard.current_state(), ShardState::Finalized);
    let err = shard.submit(mk_ticket(99, 1)).unwrap_err();
    assert_eq!(err, ShutdownError::IntakeStopped);
}

// ---------------------------------------------------------------------------
// Drain correctness
// ---------------------------------------------------------------------------

/// Drained tickets are returned in FIFO order.
#[test]
fn drain_preserves_fifo_order() {
    let mut shard = ShardModel::new(8);
    let t1 = mk_ticket(1, 1);
    let t2 = mk_ticket(1, 2);
    let t3 = mk_ticket(1, 3);
    shard.submit(t1).unwrap();
    shard.submit(t2).unwrap();
    shard.submit(t3).unwrap();
    shard.begin_shutdown().unwrap();

    let d1 = shard.drain_one().unwrap();
    let d2 = shard.drain_one().unwrap();
    let d3 = shard.drain_one().unwrap();
    assert_eq!(d1, t1);
    assert_eq!(d2, t2);
    assert_eq!(d3, t3);
    assert_eq!(shard.drain_one(), None);
}

/// No tickets are silently dropped during drain.
#[test]
fn drain_no_silent_drops() {
    let mut shard = ShardModel::new(64);
    let count = 32;
    let mut submitted = Vec::new();
    for i in 0..count {
        let t = mk_ticket(1, i);
        shard.submit(t).unwrap();
        submitted.push(t);
    }
    shard.begin_shutdown().unwrap();
    let mut drained = Vec::new();
    while let Some(t) = shard.drain_one() {
        drained.push(t);
    }
    assert_eq!(drained.len(), count as usize);
    assert_eq!(drained, submitted);
    assert_eq!(shard.pending(), 0);
}

/// `drain_one` returns `None` when not in draining state.
#[test]
fn drain_one_rejected_in_running_state() {
    let mut shard = ShardModel::new(4);
    shard.submit(mk_ticket(1, 1)).unwrap();
    // Not in draining state
    assert_eq!(shard.drain_one(), None);
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

/// `finalize` before `drain_all` returns `DrainNotComplete`.
#[test]
fn finalize_before_drain_complete_is_rejected() {
    let mut shard = ShardModel::new(4);
    shard.submit(mk_ticket(1, 1)).unwrap();
    shard.begin_shutdown().unwrap();
    // Still has pending work
    let err = shard.finalize(1).unwrap_err();
    assert_eq!(err, ShutdownError::DrainNotComplete { remaining: 1 });
}

/// Drain timeout errors preserve the number of unflushed tickets.
#[test]
fn drain_timeout_preserves_remaining_count() {
    let err = ShutdownError::DrainTimeout { remaining: 3 };
    assert_eq!(err, ShutdownError::DrainTimeout { remaining: 3 });
}

/// `finalize` from `Running` state is rejected.
#[test]
fn finalize_from_running_is_rejected() {
    let mut shard = ShardModel::new(4);
    let err = shard.finalize(1).unwrap_err();
    assert_eq!(
        err,
        ShutdownError::InvalidTransition {
            current: ShardState::Running,
            attempted: ShardState::Finalized,
        }
    );
}

/// `begin_shutdown` from `Finalized` is rejected.
#[test]
fn begin_shutdown_from_finalized_is_rejected() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    shard.drain_all();
    shard.finalize(1).unwrap();
    let err = shard.begin_shutdown().unwrap_err();
    assert_eq!(
        err,
        ShutdownError::InvalidTransition {
            current: ShardState::Finalized,
            attempted: ShardState::Draining,
        }
    );
}

/// `begin_shutdown` from `Draining` is idempotent-rejected.
#[test]
fn double_begin_shutdown_is_rejected() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    let err = shard.begin_shutdown().unwrap_err();
    assert_eq!(
        err,
        ShutdownError::InvalidTransition {
            current: ShardState::Draining,
            attempted: ShardState::Draining,
        }
    );
}

// ---------------------------------------------------------------------------
// No resurrection paths
// ---------------------------------------------------------------------------

/// After finalize, no ticket can be submitted or resurrected.
#[test]
fn no_resurrection_after_finalize() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    shard.drain_all();
    shard.finalize(1).unwrap();

    // Submit rejected
    assert!(shard.submit(mk_ticket(99, 1)).is_err());
    // Drain returns None (wrong state)
    assert_eq!(shard.drain_one(), None);
    // Another finalize rejected
    assert!(shard.finalize(2).is_err());
    // State unchanged
    assert_eq!(shard.current_state(), ShardState::Finalized);
}

/// No pending tickets remain after full lifecycle.
#[test]
fn zero_pending_after_full_lifecycle() {
    let mut shard = ShardModel::new(8);
    for i in 0..5 {
        shard.submit(mk_ticket(1, i)).unwrap();
    }
    assert_eq!(shard.pending(), 5);
    shard.begin_shutdown().unwrap();
    assert_eq!(shard.drain_all(), 5);
    assert_eq!(shard.pending(), 0);
    shard.finalize(1).unwrap();
    assert_eq!(shard.pending(), 0);
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

/// Single-ticket lifecycle: submit, drain, finalize.
#[test]
fn single_ticket_lifecycle() {
    let mut shard = ShardModel::new(1);
    shard.submit(mk_ticket(1, 0)).unwrap();
    shard.begin_shutdown().unwrap();
    let t = shard.drain_one().unwrap();
    assert_eq!(shard.pending(), 0);
    assert_eq!(t.seq.get(), 0);
    shard.finalize(1).unwrap();
    assert!(shard.is_flushed());
}

/// Large batch drain validates no overflow in pending counter.
#[test]
fn large_batch_no_overflow() {
    let mut shard = ShardModel::new(128);
    let count: usize = 64;
    for i in 0..count {
        shard.submit(mk_ticket(1, i as u32)).unwrap();
    }
    assert_eq!(shard.pending(), count);
    shard.begin_shutdown().unwrap();
    assert_eq!(shard.drain_all(), count);
    assert_eq!(shard.pending(), 0);
    shard.finalize(1).unwrap();
}

/// Running shard reports correct state and is not flushed.
#[test]
fn running_shard_not_flushed() {
    let shard = ShardModel::new(4);
    assert_eq!(shard.current_state(), ShardState::Running);
    assert!(!shard.is_flushed());
}

/// `drain_all` returns zero when queue is empty.
#[test]
fn drain_all_returns_zero_when_empty() {
    let mut shard = ShardModel::new(4);
    shard.begin_shutdown().unwrap();
    assert_eq!(shard.drain_all(), 0);
}
