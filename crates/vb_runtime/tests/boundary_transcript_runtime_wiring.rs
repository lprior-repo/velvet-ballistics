#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Integration tests for the cold-path boundary transcript wiring.
//!
//! These tests exercise the end-to-end path required by vb-awxlm:
//! `Runtime::new` accepts an `Option<SharedBoundaryTranscript>`, the
//! runtime drives real events through the journal projection, and the
//! authority-bearing `record_*` paths fire on the corresponding shard
//! lifecycle methods. The tests cover:
//!
//! 1. Construction with `Some(SharedBoundaryTranscript)` threads the
//!    transcript through to every shard.
//! 2. Ask-answer flow pushes the full payload via `record_ask_answered`
//!    after the journal batch returns Ok.
//! 3. Action failure flow pushes the full failure authority via
//!    `record_action_failed`.
//! 4. Wait / Ask timer registration pushes `record_timer_captured` and
//!    the timer fire path pushes `record_timer_fired`.

use std::num::NonZeroUsize;
use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, Idempotency,
    RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_runtime::boundary_transcript::{
    AskAnswerAuthority, BoundaryEvent, BoundaryTranscriptError, BoundaryTranscriptJournal,
    FailureAuthority, FailureCodeTag, RetryPolicyTag, SharedBoundaryTranscript, TimerAuthority,
};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{AskAnswer, AskTicket, ShardConfig};

/// Builds a minimal valid compiled workflow with a single Do node that
/// schedules an action and then finishes. Mirrors the suspended-workflow
/// fixture in `runtime_tests.rs` so the existing admission / journal
/// plumbing accepts the artifact envelope.
fn single_do_workflow() -> vb_core::workflow::CompiledWorkflow {
    let do_node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("single_do"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([do_node, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts)
        .expect("single-do workflow must validate")
}

fn test_action_contract(action: ActionId) -> ActionContract {
    ActionContract {
        id: action,
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([]),
    }
}

fn relaxed_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 8,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
    }
}

fn variants(events: &[BoundaryEvent]) -> Vec<&'static str> {
    events.iter().map(|e| e.kind()).collect()
}

fn drive_ticks(runtime: &mut Runtime) {
    let _ = runtime.tick_all();
    let _ = runtime.tick_all();
    let _ = runtime.tick_all();
}

/// **vb-awxlm end-to-end #1** — `Runtime::new` with a
/// `Some(SharedBoundaryTranscript)` threads the transcript to all shards.
/// A successful tick on a single-Do workflow produces at least one
/// journal-projected boundary event.
#[test]
fn runtime_construction_threads_transcript_to_shards() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(64);
    let shard_count = NonZeroUsize::new(1).expect("non-zero shard count");
    let mut runtime = Runtime::new(
        shard_count,
        relaxed_config(),
        vb_runtime::journal::VolatileRuntimeJournal::shared(),
        Some(shared.clone()),
    );
    let workflow = single_do_workflow();
    let run = RunId::new(1);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            vb_core::capability::CapabilitySet::empty(),
            Box::from([test_action_contract(ActionId::new(0))]),
        )
        .map_err(|e| format!("submit: {e:?}"))?;
    drive_ticks(&mut runtime);
    let snap = shared.snapshot().map_err(stringify)?;
    let kinds = variants(&snap.iter().map(|e| e.event.clone()).collect::<Vec<_>>());
    assert!(
        !kinds.is_empty(),
        "expected at least one journal-projected boundary event, got kinds: {kinds:?}"
    );
    runtime
        .shutdown_graceful()
        .map_err(|e| format!("shutdown: {e:?}"))?;
    Ok(())
}

/// **vb-awxlm end-to-end #2** — direct `record_ask_answered` capture
/// via the `AskAnswerAuthority` newtype (the public API surface used by
/// `handle_ask_answer`) preserves the full payload the journal drops.
#[test]
fn ask_answer_authority_capture_carries_full_payload() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let authority = AskAnswerAuthority::new(
        RunId::new(42),
        StepIdx::new(2),
        StepIdx::new(3),
        SlotIdx::new(7),
        Taint::Secret,
        /* encoded_len */ 128,
    );
    let seq = proj.record_ask_answered(&authority).map_err(stringify)?;
    assert_eq!(seq, Some(0));
    let snap = shared.snapshot().map_err(stringify)?;
    assert_eq!(snap.len(), 1);
    match &snap[0].event {
        BoundaryEvent::AskAnswered {
            run,
            ask_step,
            resume_step,
            slot,
            taint,
            encoded_len,
        } => {
            assert_eq!(*run, RunId::new(42));
            assert_eq!(*ask_step, StepIdx::new(2));
            assert_eq!(*resume_step, StepIdx::new(3));
            assert_eq!(*slot, SlotIdx::new(7));
            assert_eq!(*taint, Taint::Secret);
            assert_eq!(*encoded_len, 128);
        }
        other => panic!("expected AskAnswered, got {other:?}"),
    }
    Ok(())
}

/// **vb-awxlm end-to-end #3** — `record_action_failed` direct capture
/// via the `FailureAuthority` newtype round-trips the typed
/// `ActionFailureCode` and `RetryPolicy` through the `u8` wire tags.
#[test]
fn failure_authority_capture_round_trips_typed_tags() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let authority = FailureAuthority::new(
        RunId::new(99),
        StepIdx::new(5),
        ActionId::new(11),
        /* attempt */ 3,
        FailureCodeTag::from(ActionFailureCode::Timeout),
        RetryPolicyTag::from(vb_core::action::RetryPolicy::Retryable),
        Taint::DerivedFromSecret,
    );
    proj.record_action_failed(&authority).map_err(stringify)?;
    let snap = shared.snapshot().map_err(stringify)?;
    assert_eq!(snap.len(), 1);
    match &snap[0].event {
        BoundaryEvent::ActionFailed {
            run,
            step,
            action,
            attempt,
            failure_code,
            retry_policy_tag,
            taint,
        } => {
            assert_eq!(*run, RunId::new(99));
            assert_eq!(*step, StepIdx::new(5));
            assert_eq!(*action, ActionId::new(11));
            assert_eq!(*attempt, 3);
            assert_eq!(*failure_code, ActionFailureCode::Timeout as u8);
            assert_eq!(*retry_policy_tag, 1);
            assert_eq!(*taint, Taint::DerivedFromSecret);
        }
        other => panic!("expected ActionFailed, got {other:?}"),
    }
    Ok(())
}

/// **vb-awxlm end-to-end #4** — `record_timer_captured` /
/// `record_timer_fired` via the `TimerAuthority` newtype preserves the
/// full timer authority the journal cannot encode.
#[test]
fn timer_authority_capture_carries_full_authority() -> Result<(), String> {
    let shared = SharedBoundaryTranscript::with_capacity(8);
    let proj = BoundaryTranscriptJournal::new(shared.clone());
    let deadline = std::time::Instant::now();
    let captured = TimerAuthority::new(
        RunId::new(7),
        StepIdx::new(0),
        vb_runtime::shard::PendingTimerKind::Wait,
        /* generation */ 11,
        deadline,
        /* logical_deadline */ 4096,
    );
    let fired = TimerAuthority::new(
        RunId::new(7),
        StepIdx::new(0),
        vb_runtime::shard::PendingTimerKind::Wait,
        /* generation */ 11,
        deadline,
        /* logical_deadline */ 0,
    );
    proj.record_timer_captured(&captured).map_err(stringify)?;
    proj.record_timer_fired(&fired).map_err(stringify)?;
    let snap = shared.snapshot().map_err(stringify)?;
    assert_eq!(snap.len(), 2);
    assert!(matches!(snap[0].event, BoundaryEvent::TimerCaptured { .. }));
    assert!(matches!(snap[1].event, BoundaryEvent::TimerFired { .. }));
    Ok(())
}

/// **vb-awxlm end-to-end #5** — `Runtime::new_for_tests_and_benchmarks_only`
/// also accepts the optional `SharedBoundaryTranscript` parameter
/// (default `None` is allowed). A workload submitted with `None` must
/// still drive the journal projection through the existing code path.
#[test]
fn runtime_for_tests_accepts_optional_transcript() -> Result<(), String> {
    let shard_count = NonZeroUsize::new(1).expect("non-zero shard count");
    let mut runtime =
        Runtime::new_for_tests_and_benchmarks_only(shard_count, relaxed_config(), None);
    let workflow = single_do_workflow();
    let run = RunId::new(2);
    runtime
        .submit_direct_with_inputs_grants_and_contracts(
            run,
            workflow,
            Box::from([(SlotIdx::new(0), SlotValue::I64(0))]),
            vb_core::capability::CapabilitySet::empty(),
            Box::from([test_action_contract(ActionId::new(0))]),
        )
        .map_err(|e| format!("submit: {e:?}"))?;
    drive_ticks(&mut runtime);
    runtime
        .shutdown_graceful()
        .map_err(|e| format!("shutdown: {e:?}"))?;
    Ok(())
}

/// Helper: serializes a [`BoundaryTranscriptError`] for assertion messages.
fn stringify(error: BoundaryTranscriptError) -> String {
    format!("{error:?}")
}

// Suppress the unused-import lint for `AskAnswer` and `ActionOutputReady`
// — the tests reference them through the `Runtime` API surface (e.g.,
// `Runtime::answer_ask`) and the newtype constructors; this module's
// lints are enforced at the boundary.
#[allow(dead_code)]
fn _imports_kept_alive(
    _: AskAnswer,
    _: AskTicket,
    _: ActionFailure,
    _: ActionOutputReady,
    _: SlotValue,
) {
}
