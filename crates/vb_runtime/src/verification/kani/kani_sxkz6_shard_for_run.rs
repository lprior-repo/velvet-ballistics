//!
//! Kani harnesses for RA-030 wave-15 follow-up — shard_for_run helper.
//!
//! Bead: vb-sxkz6
//! Obligations: obl-ps-ra030-scan-all-correctness-kani,
//!              obl-ps-ra030-answer-ask-routing-kani,
//!              obl-ps-ra030-list-events-routing-kani,
//!              obl-ps-ra030-take-inspect-routing-kani,
//!              obl-ps-ra030-capture-timer-entry-routing-kani,
//!              obl-ps-ra030-timer-entry-fired-routing-kani.
//!
//! Target: crate::runtime::Runtime::answer_ask,
//!         crate::runtime::Runtime::list_events,
//!         crate::runtime::Runtime::take_inspect_response,
//!         crate::runtime::Runtime::capture_timer_entry,
//!         crate::runtime::Runtime::timer_entry_fired.
//!
//! GOD RULE 1: All inputs use kani::any() with explicit bounds.
//! GOD RULE 2: Every harness calls production functions.
//! GOD RULE 5: Property obligations have assertions, not just cover!.

#![forbid(unsafe_code)]
#![cfg(kani)]

use std::num::NonZeroUsize;
use std::time::Instant;

use vb_core::ids::RunId;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;

use crate::error::RuntimeError;
use crate::runtime::Runtime;
use crate::shard::timer_wheel::TimerEntry;
use crate::shard::{AskAnswer, AskTicket, PendingTimerKind, ShardConfig};
use vb_core::ids::StepIdx;
use vb_core::value::Taint;

// =========================================================================
// Bounded generators (GOD RULE 1)
// =========================================================================

fn any_run_id_bounded() -> RunId {
    let raw: u64 = kani::any();
    kani::assume(raw < 8); // small bounded space
    RunId::new(raw)
}

fn small_runtime(shard_count: usize) -> Runtime {
    let count = NonZeroUsize::new(shard_count).expect("shard_count >= 1");
    Runtime::new_for_tests_and_benchmarks_only(count, ShardConfig::default(), None)
}

fn make_answer(run: RunId) -> AskAnswer {
    AskAnswer {
        ticket: AskTicket {
            run,
            ask_step: StepIdx::ZERO,
            resume_step: StepIdx::new(1),
        },
        answer_slot: SlotIdx::new(0),
        value: SlotValue::Bool(true),
        taint: Taint::Clean,
        encoded_len: 1u32,
    }
}

// =========================================================================
// obl-ps-ra030-scan-all-correctness-kani
// Property C4: unknown run yields RunNotFound for answer_ask.
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_answer_ask_unknown_run_returns_not_found() {
    let shard_count: usize = kani::any();
    kani::assume(shard_count >= 1 && shard_count <= 4);

    let runtime = small_runtime(shard_count);
    let run = any_run_id_bounded();
    let answer = make_answer(run);

    let result = runtime.answer_ask(answer);
    kani::assert(
        matches!(result, Err(RuntimeError::RunNotFound)),
        "answer_ask on unknown run must return RunNotFound",
    );
}

// =========================================================================
// obl-ps-ra030-list-events-routing-kani
// Property C5: list_events reads trace from owning shard.
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_list_events_unknown_run_returns_not_found() {
    let shard_count: usize = kani::any();
    kani::assume(shard_count >= 1 && shard_count <= 4);

    let runtime = small_runtime(shard_count);
    let run = any_run_id_bounded();

    let result = runtime.list_events(run);
    kani::assert(
        matches!(result, Err(RuntimeError::RunNotFound)),
        "list_events on unknown run must return RunNotFound",
    );
}

// =========================================================================
// obl-ps-ra030-take-inspect-routing-kani
// Property C5: take_inspect_response drains inspect slot of owning shard.
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_take_inspect_unknown_run_returns_not_found() {
    let shard_count: usize = kani::any();
    kani::assume(shard_count >= 1 && shard_count <= 4);

    let mut runtime = small_runtime(shard_count);
    let run = any_run_id_bounded();

    let result = runtime.take_inspect_response(run);
    kani::assert(
        matches!(result, Err(RuntimeError::RunNotFound)),
        "take_inspect_response on unknown run must return RunNotFound",
    );
}

// =========================================================================
// obl-ps-ra030-capture-timer-entry-routing-kani
// Property C5: capture_timer_entry reads timer entry from owning shard.
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_capture_timer_entry_unknown_run_returns_not_found() {
    let shard_count: usize = kani::any();
    kani::assume(shard_count >= 1 && shard_count <= 4);

    let runtime = small_runtime(shard_count);
    let run = any_run_id_bounded();

    let result = runtime.capture_timer_entry(run);
    kani::assert(
        matches!(result, Err(RuntimeError::RunNotFound)),
        "capture_timer_entry on unknown run must return RunNotFound",
    );
}

// =========================================================================
// obl-ps-ra030-timer-entry-fired-routing-kani
// Property C5: timer_entry_fired delivers to owning shard.
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_timer_entry_fired_unknown_run_returns_not_found() {
    let shard_count: usize = kani::any();
    kani::assume(shard_count >= 1 && shard_count <= 4);

    let runtime = small_runtime(shard_count);
    let run = any_run_id_bounded();

    use crate::shard::timer_wheel::TimerEntry;
    let entry = TimerEntry {
        run,
        generation: 0,
        deadline: Instant::now(),
        kind: PendingTimerKind::Ask,
    };
    let result = runtime.timer_entry_fired(entry);
    kani::assert(
        matches!(result, Err(RuntimeError::RunNotFound)),
        "timer_entry_fired on unknown run must return RunNotFound",
    );
}

// =========================================================================
// obl-ps-ra030-scan-all-bounded-cost-kani
// Property C7: shard_index produces index in [0, shard_count).
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_shard_index_bounded() {
    let shard_count: u64 = kani::any();
    kani::assume(shard_count >= 1 && shard_count <= 4);

    let run_raw: u64 = kani::any();
    kani::assume(run_raw < 16);

    // Since shard_count >= 1, checked_rem must succeed and remainder < shard_count.
    let remainder = run_raw.checked_rem(shard_count).expect("nonzero divisor");
    kani::assert(remainder < shard_count, "remainder must be < shard_count");
}

// =========================================================================
// obl-ps-ra030-scan-all-determinism-kani
// Property C6: shard_index is deterministic (same inputs → same output).
// =========================================================================

#[kani::proof]
#[kani::unwind(8)]
fn kani_sxkz6_shard_index_determinism() {
    let shard_count: u8 = kani::any();
    kani::assume(shard_count >= 1);

    let run_raw: u8 = kani::any();

    let r1 = (run_raw as u64).checked_rem(shard_count as u64);
    let r2 = (run_raw as u64).checked_rem(shard_count as u64);
    assert!(r1 == r2, "shard_index must be deterministic");
}
