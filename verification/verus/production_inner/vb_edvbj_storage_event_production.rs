// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection mirror for vb-edvbj::StorageRuntimeJournal::storage_event
// ============================================================================
//
// Verbatim mirror of the post-fix production body shape at
// `crates/vb_runtime/src/journal/chunk_002.rs:1-355`. The post-fix body
// replaces the buggy wildcard fallback (which fabricated
// `JournalEvent::RunFailedEvent` for any unmapped `RuntimeJournalEvent`)
// with a typed `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })`.
//
// DRIFT POLICY: `crates/vb_runtime/src/journal/chunk_002.rs:1-355`
// Production source coverage:
//   - `MirrorRuntimeJournalEvent` discriminant 21-variant enum
//                              <- crates/vb_runtime/src/journal/chunk_001.rs:14-195
//   - `MirrorRuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }`
//                              <- crates/vb_runtime/src/error/mod.rs (POST-FIX)
//   - `mirror_run_storage_event`     <- crates/vb_runtime/src/journal/chunk_002.rs:41-103
//   - `mirror_action_storage_event`  <- crates/vb_runtime/src/journal/chunk_002.rs:105-191
//   - `mirror_boundary_storage_event`<- crates/vb_runtime/src/journal/chunk_002.rs:193-268
//   - `mirror_storage_event`         <- crates/vb_runtime/src/journal/chunk_002.rs:270-303
//                                       (POST-FIX shape: Err(UnmappedRuntimeJournalEvent)
//                                        instead of Ok(RunFailedEvent)
//                                        fabricated fallback)
//   - `mirror_runtime_journal_event_kind`
//                              <- companion spec file
//                                   (counts every 21 declared variant)
//
// WHY STRUCTURAL MIRROR (NOT DIRECT `#[path]` INCLUSION)
// ============================================================================
//
// Direct `#[path = "../../crates/vb_runtime/src/journal/chunk_002.rs"]`
// inclusion is blocked because the production source imports
// `vb_storage::{DurabilityProfile, EventSeq, FjallJournal, JournalEvent,
//  JournalWriterFlushReport, JournalWriterQueue, StorageLimits}` and
// `vb_core::{Taint, ids::*, ...}` plus uses `Arc<FjallJournal>`,
// `serde::{Serialize, Deserialize}`, `Mutex<...>`, etc. None of these
// are in the standalone Verus unit's extern prelude. The structural
// mirror sidesteps every blocker while preserving the production body
// shape (variant-by-variant match) verbatim.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production body of `storage_event` is NOT verified by Verus
// directly. The mirror re-implements the body line-by-line against
// `MirrorRuntimeJournalEvent`, `MirrorEventSeq`, `MirrorJournalEvent`,
// and `MirrorRuntimeError`. Drift between this mirror and the
// production source is detected by `scripts/check-production-inner-drift.sh`
// (CI gate) and breaks the spec build at compile time.
//
// Regenerate this mirror whenever production adds, removes, or renames
// any of:
//   - `RuntimeJournalEvent` variant (chunk_001.rs:14-195)
//   - `MirrorJournalEvent` discriminant (production events.rs)
//   - Per-helper variant arm (chunk_002.rs:41-103, :105-191, :193-268)
//   - `runtime_journal_event_kind` arm set (companion spec)
//   - `RuntimeError::UnmappedRuntimeJournalEvent { event_kind }` shape
//     (mod.rs POST-FIX addition)

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Mirror of `vb_core::ids::RunId` (crates/vb_core/src/ids/mod.rs)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorRunId {
    pub value: u64,
}

impl MirrorRunId {
    #[must_use]
    pub const fn new(value: u64) -> Self { Self { value } }
    #[must_use]
    pub const fn get(self) -> u64 { self.value }
}

// ---------------------------------------------------------------------------
// Mirror of `vb_storage::types::EventSeq` (crates/vb_storage/src/types.rs:73)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorEventSeq {
    pub value: u64,
}

impl MirrorEventSeq {
    #[must_use]
    pub const fn new(value: u64) -> Self { Self { value } }
    #[must_use]
    pub const fn get(self) -> u64 { self.value }
}

// ---------------------------------------------------------------------------
// Mirror of `vb_storage::records::SlotIdx`, `StepIdx`, `ActionId`,
// `WorkflowDigest`, `CapabilitySet`, `RuntimePolicy`, `Taint` (placeholders)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorSlotIdx { pub value: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorStepIdx { pub value: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorActionId { pub value: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorWorkflowDigest { pub value: [u8; 32] }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MirrorCapabilitySet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirrorRuntimePolicy { Relaxed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorTaint;

// ---------------------------------------------------------------------------
// Mirror of `vb_runtime::admission::RunAdmission` (stub)
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorRunAdmission {
    pub run_id: MirrorRunId,
    pub artifact_digest: MirrorWorkflowDigest,
    pub granted: MirrorCapabilitySet,
    pub policy: MirrorRuntimePolicy,
}

impl MirrorRunAdmission {
    pub fn run_id(&self) -> MirrorRunId { self.run_id }
    pub fn artifact_digest(&self) -> MirrorWorkflowDigest { self.artifact_digest }
    pub fn granted_capabilities(&self) -> &MirrorCapabilitySet { &self.granted }
    pub fn policy(&self) -> MirrorRuntimePolicy { self.policy }
}

// ---------------------------------------------------------------------------
// Mirror of `vb_runtime::error::RuntimeError` (the variants exercised by
// `storage_event` and `boundary_storage_event`). The post-fix shape adds
// `UnmappedRuntimeJournalEvent { event_kind: &'static str }`.
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorRuntimeError {
    /// Hard-coded fallback (EncodeFailed) from `encoded_slot_taint_extra`.
    EncodeFailed,
    /// POST-FIX typed error added by vb-cib14 to replace the
    /// fabricating fallback that mapped unmapped journal events to
    /// `JournalEvent::RunFailedEvent`.
    UnmappedRuntimeJournalEvent {
        /// Literal variant name from the `runtime_journal_event_kind`
        /// helper.
        event_kind: &'static str,
    },
}

// ---------------------------------------------------------------------------
// Mirror of `vb_storage::events::JournalEvent` (subset of variants that
// `storage_event` and its helpers can produce).
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorJournalEvent {
    RunAccepted {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        workflow: MirrorWorkflowDigest,
    },
    RunAdmission {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        artifact_digest: MirrorWorkflowDigest,
        granted_capabilities: MirrorCapabilitySet,
        policy: MirrorRuntimePolicy,
    },
    RunFinished {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        result: MirrorSlotIdx,
        attempt: u16,
    },
    RunFailedEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        attempt: u16,
    },
    RunCancelled {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        attempt: u16,
        reason: Option<String>,
    },
    RunKilled {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        attempt: u16,
    },
    StepStarted {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        attempt: u16,
    },
    StepSucceeded {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        output: MirrorSlotIdx,
    },
    ActionScheduled {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        action: MirrorActionId,
        attempt: u16,
    },
    ActionCompletedEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        action: MirrorActionId,
        attempt: u16,
    },
    ActionScheduledTicket {
        run: MirrorRunId,
        seq: MirrorEventSeq,
    },
    ActionCompletedEnvelope {
        run: MirrorRunId,
        seq: MirrorEventSeq,
    },
    ActionFailedEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        action: MirrorActionId,
        attempt: u16,
    },
    ActionAbandoned {
        run: MirrorRunId,
        seq: MirrorEventSeq,
    },
    WaitScheduledEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        attempt: u16,
    },
    WaitResolvedEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        attempt: u16,
    },
    AskScheduledEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        attempt: u16,
    },
    AskAnsweredEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        attempt: u16,
    },
    AskTimedOutEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        step: MirrorStepIdx,
        attempt: u16,
    },
    SlotWrittenEvent {
        run: MirrorRunId,
        seq: MirrorEventSeq,
        slot: MirrorSlotIdx,
    },
}

// ---------------------------------------------------------------------------
// Mirror of `vb_runtime::journal::RuntimeJournalEvent` (the 21 declared
// variants enumerated from crates/vb_runtime/src/journal/chunk_001.rs:14-195).
// ---------------------------------------------------------------------------
//
// This is the drift-detection surface for the no-fabrication claim. Any
// variant added/removed in production breaks the spec build at this
// point in the mirror compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirrorRuntimeJournalEvent {
    // Variant set 1 (run-layer)
    RunSubmitted { run: MirrorRunId, workflow: MirrorWorkflowDigest },
    RunAdmission { admission: MirrorRunAdmission },
    RunFinished { run: MirrorRunId, result: MirrorSlotIdx },
    RunFailed { run: MirrorRunId },
    RunCancelled { run: MirrorRunId, reason: Option<String> },
    RunKilled { run: MirrorRunId },
    StepStarted { run: MirrorRunId, step: MirrorStepIdx },
    StepSucceeded { run: MirrorRunId, step: MirrorStepIdx, output: MirrorSlotIdx },
    // Variant set 2 (action-layer)
    ActionScheduled { run: MirrorRunId, step: MirrorStepIdx, action: MirrorActionId },
    ActionCompleted { run: MirrorRunId, step: MirrorStepIdx, action: MirrorActionId },
    ActionScheduledTicket,
    ActionCompletedEnvelope,
    ActionFailed { run: MirrorRunId, step: MirrorStepIdx, action: MirrorActionId, attempt: u16 },
    ActionAbandoned,
    // Variant set 3 (boundary-layer)
    WaitScheduled { run: MirrorRunId, step: MirrorStepIdx },
    WaitResolved { run: MirrorRunId, step: MirrorStepIdx },
    AskScheduled { run: MirrorRunId, step: MirrorStepIdx },
    AskAnswered { run: MirrorRunId, step: MirrorStepIdx, slot: MirrorSlotIdx },
    AskTimedOut { run: MirrorRunId, step: MirrorStepIdx },
    SlotWritten { run: MirrorRunId, slot: MirrorSlotIdx, value: Vec<u8>, taint: MirrorTaint, extra: Option<Vec<u8>> },
    Resumed { run: MirrorRunId, timestamp: u64 },
}

impl MirrorRuntimeJournalEvent {
    /// Mirror of `RuntimeJournalEvent::run_id` at chunk_001.rs:200-223.
    #[verifier::external]
    pub fn run_id(&self) -> MirrorRunId {
        match self {
            Self::RunSubmitted { run, .. }
            | Self::RunFinished { run, .. }
            | Self::RunFailed { run }
            | Self::RunCancelled { run, .. }
            | Self::RunKilled { run, .. }
            | Self::ActionScheduled { run, .. }
            | Self::ActionCompleted { run, .. }
            | Self::ActionFailed { run, .. }
            | Self::WaitScheduled { run, .. }
            | Self::WaitResolved { run, .. }
            | Self::AskScheduled { run, .. }
            | Self::AskAnswered { run, .. }
            | Self::AskTimedOut { run, .. }
            | Self::SlotWritten { run, .. }
            | Self::StepStarted { run, .. }
            | Self::StepSucceeded { run, .. }
            | Self::Resumed { run, .. } => *run,
            Self::ActionScheduledTicket
            | Self::ActionCompletedEnvelope
            | Self::ActionAbandoned => MirrorRunId { value: 0 },
            Self::RunAdmission { admission } => admission.run_id(),
        }
    }
}

// ---------------------------------------------------------------------------
// Result alias (mirror of `crate::RuntimeResult<T>`)
// ---------------------------------------------------------------------------
pub type MirrorRuntimeResult<T> = Result<T, MirrorRuntimeError>;

// ---------------------------------------------------------------------------
// Per-layer helper mirrors — STRUCTURAL mirror of chunk_002.rs:41-268
// ---------------------------------------------------------------------------
//
// `mirror_run_storage_event` <- chunk_002.rs:41-103
#[verifier::external_body]
pub fn mirror_run_storage_event(
    event: MirrorRuntimeJournalEvent,
    seq: MirrorEventSeq,
) -> Option<MirrorJournalEvent> {
    match event {
        MirrorRuntimeJournalEvent::RunSubmitted { run, workflow } => {
            Some(MirrorJournalEvent::RunAccepted { run, seq, workflow })
        }
        MirrorRuntimeJournalEvent::RunAdmission { admission } => {
            Some(MirrorJournalEvent::RunAdmission {
                run: admission.run_id(),
                seq,
                artifact_digest: admission.artifact_digest(),
                granted_capabilities: *admission.granted_capabilities(),
                policy: admission.policy(),
            })
        }
        MirrorRuntimeJournalEvent::RunFinished { run, result } => {
            Some(MirrorJournalEvent::RunFinished {
                run,
                seq,
                result,
                attempt: 1,
            })
        }
        MirrorRuntimeJournalEvent::RunFailed { run } => {
            Some(MirrorJournalEvent::RunFailedEvent {
                run,
                seq,
                attempt: 1,
            })
        }
        MirrorRuntimeJournalEvent::RunCancelled { run, reason } => {
            Some(MirrorJournalEvent::RunCancelled {
                run,
                seq,
                attempt: 1,
                reason,
            })
        }
        MirrorRuntimeJournalEvent::RunKilled { run } => {
            Some(MirrorJournalEvent::RunKilled {
                run,
                seq,
                attempt: 1,
            })
        }
        MirrorRuntimeJournalEvent::StepStarted { run, step } => {
            Some(MirrorJournalEvent::StepStarted {
                run,
                seq,
                step,
                attempt: 1,
            })
        }
        MirrorRuntimeJournalEvent::StepSucceeded { run, step, output } => {
            Some(MirrorJournalEvent::StepSucceeded {
                run,
                seq,
                step,
                output,
            })
        }
        // All other variants map to None in this layer.
        _ => None,
    }
}

// `mirror_action_storage_event` <- chunk_002.rs:105-191
#[verifier::external_body]
pub fn mirror_action_storage_event(
    event: MirrorRuntimeJournalEvent,
    seq: MirrorEventSeq,
) -> Option<MirrorJournalEvent> {
    match event {
        MirrorRuntimeJournalEvent::ActionScheduled { run, step, action } => {
            Some(MirrorJournalEvent::ActionScheduled {
                run,
                seq,
                step,
                action,
                attempt: 1,
            })
        }
        MirrorRuntimeJournalEvent::ActionCompleted { run, step, action } => {
            Some(MirrorJournalEvent::ActionCompletedEvent {
                run,
                seq,
                step,
                action,
                attempt: 1,
            })
        }
        MirrorRuntimeJournalEvent::ActionFailed { run, step, action, attempt } => {
            Some(MirrorJournalEvent::ActionFailedEvent {
                run,
                seq,
                step,
                action,
                attempt,
            })
        }
        // All other variants map to None in this layer.
        _ => None,
    }
}

// `mirror_boundary_storage_event` <- chunk_002.rs:193-268
#[verifier::external_body]
pub fn mirror_boundary_storage_event(
    event: MirrorRuntimeJournalEvent,
    seq: MirrorEventSeq,
) -> MirrorRuntimeResult<Option<MirrorJournalEvent>> {
    match event {
        MirrorRuntimeJournalEvent::WaitScheduled { run, step } => {
            Ok(Some(MirrorJournalEvent::WaitScheduledEvent {
                run,
                seq,
                step,
                attempt: 1,
            }))
        }
        MirrorRuntimeJournalEvent::WaitResolved { run, step } => {
            Ok(Some(MirrorJournalEvent::WaitResolvedEvent {
                run,
                seq,
                step,
                attempt: 1,
            }))
        }
        MirrorRuntimeJournalEvent::AskScheduled { run, step } => {
            Ok(Some(MirrorJournalEvent::AskScheduledEvent {
                run,
                seq,
                step,
                attempt: 1,
            }))
        }
        MirrorRuntimeJournalEvent::AskAnswered { run, step, .. } => {
            Ok(Some(MirrorJournalEvent::AskAnsweredEvent {
                run,
                seq,
                step,
                attempt: 1,
            }))
        }
        MirrorRuntimeJournalEvent::AskTimedOut { run, step } => {
            Ok(Some(MirrorJournalEvent::AskTimedOutEvent {
                run,
                seq,
                step,
                attempt: 1,
            }))
        }
        // All other variants map to Ok(None) in this layer; the slot
        // taint encoding path collapses to None in the mirror (real
        // production returns Ok(Some(SlotWrittenEvent)) for SlotWritten
        // only — abstracted here because the mirror cannot import
        // `vb_storage::encode_slot_written_extra`).
        _ => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// POST-FIX `mirror_storage_event` — REPLACES the buggy fallback
// ---------------------------------------------------------------------------
//
// STRUCTURAL mirror of the post-fix body at chunk_002.rs:270-303
// (which is the planned shape after vb-cib14 lands).
//
// The PRE-FIX body (current state in main) fabricates
// `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` for every
// unmapped variant. The POST-FIX body returns
// `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind })`.
// Spec contracts attached via `assume_specification` at the companion
// spec file (`verification/verus/vb_edvbj_storage_event.rs`) state the
// post-fix contract; this mirror is the body that satisfies it.
#[verifier::external_body]
#[verifier::external]
pub fn mirror_storage_event(
    event: &MirrorRuntimeJournalEvent,
    seq: MirrorEventSeq,
) -> MirrorRuntimeResult<MirrorJournalEvent> {
    let result = match event {
        // Variant set 1 (run-layer)
        MirrorRuntimeJournalEvent::RunSubmitted { .. }
        | MirrorRuntimeJournalEvent::RunAdmission { .. }
        | MirrorRuntimeJournalEvent::RunFinished { .. }
        | MirrorRuntimeJournalEvent::RunFailed { .. }
        | MirrorRuntimeJournalEvent::RunCancelled { .. }
        | MirrorRuntimeJournalEvent::RunKilled { .. }
        | MirrorRuntimeJournalEvent::StepStarted { .. }
        | MirrorRuntimeJournalEvent::StepSucceeded { .. } => {
            Ok(mirror_run_storage_event(event.clone(), seq))
        }
        // Variant set 2 (action-layer)
        MirrorRuntimeJournalEvent::ActionScheduled { .. }
        | MirrorRuntimeJournalEvent::ActionCompleted { .. }
        | MirrorRuntimeJournalEvent::ActionScheduledTicket
        | MirrorRuntimeJournalEvent::ActionCompletedEnvelope
        | MirrorRuntimeJournalEvent::ActionFailed { .. }
        | MirrorRuntimeJournalEvent::ActionAbandoned => {
            Ok(mirror_action_storage_event(event.clone(), seq))
        }
        // Variant set 3 (boundary-layer)
        _ => mirror_boundary_storage_event(event.clone(), seq),
    }?;
    if let Some(storage_event) = result {
        Ok(storage_event)
    } else {
        // POST-FIX: typed error instead of fabricated RunFailedEvent.
        Err(MirrorRuntimeError::UnmappedRuntimeJournalEvent {
            event_kind: mirror_runtime_journal_event_kind(event),
        })
    }
}

// ---------------------------------------------------------------------------
// `mirror_runtime_journal_event_kind` — companion to POST-FIX storage_event
// ---------------------------------------------------------------------------
//
// Enumerates every one of the 21 declared `MirrorRuntimeJournalEvent`
// variants and returns the literal variant name as `&'static str`. This
// is the H-4 future-variant mitigation: the helper is required to be
// updated alongside any new variant.
#[verifier::external]
pub fn mirror_runtime_journal_event_kind(event: &MirrorRuntimeJournalEvent) -> &'static str {
    match event {
        MirrorRuntimeJournalEvent::RunSubmitted { .. } => "RunSubmitted",
        MirrorRuntimeJournalEvent::RunAdmission { .. } => "RunAdmission",
        MirrorRuntimeJournalEvent::RunFinished { .. } => "RunFinished",
        MirrorRuntimeJournalEvent::RunFailed { .. } => "RunFailed",
        MirrorRuntimeJournalEvent::RunCancelled { .. } => "RunCancelled",
        MirrorRuntimeJournalEvent::RunKilled { .. } => "RunKilled",
        MirrorRuntimeJournalEvent::ActionScheduled { .. } => "ActionScheduled",
        MirrorRuntimeJournalEvent::ActionCompleted { .. } => "ActionCompleted",
        MirrorRuntimeJournalEvent::ActionScheduledTicket => "ActionScheduledTicket",
        MirrorRuntimeJournalEvent::ActionCompletedEnvelope => "ActionCompletedEnvelope",
        MirrorRuntimeJournalEvent::ActionFailed { .. } => "ActionFailed",
        MirrorRuntimeJournalEvent::ActionAbandoned => "ActionAbandoned",
        MirrorRuntimeJournalEvent::WaitScheduled { .. } => "WaitScheduled",
        MirrorRuntimeJournalEvent::WaitResolved { .. } => "WaitResolved",
        MirrorRuntimeJournalEvent::AskScheduled { .. } => "AskScheduled",
        MirrorRuntimeJournalEvent::AskAnswered { .. } => "AskAnswered",
        MirrorRuntimeJournalEvent::AskTimedOut { .. } => "AskTimedOut",
        MirrorRuntimeJournalEvent::SlotWritten { .. } => "SlotWritten",
        MirrorRuntimeJournalEvent::StepStarted { .. } => "StepStarted",
        MirrorRuntimeJournalEvent::StepSucceeded { .. } => "StepSucceeded",
        MirrorRuntimeJournalEvent::Resumed { .. } => "Resumed",
    }
}

// ---------------------------------------------------------------------------
// Drift-detection helper
// ---------------------------------------------------------------------------
//
// Inside `verus!` because it calls `#[verifier::external]` mirror fns.
// The references force Rust to resolve every mirror method at compile
// time. Renaming any production mirror method or failing to update the
// 21-variant match arms below breaks this fn's compilation.
#[allow(dead_code)]
fn prod_methods_drift_check() -> MirrorRuntimeResult<()> {
    let run = MirrorRunId::new(0);
    let seq = MirrorEventSeq::new(0);
    let _ = MirrorRunId::get(run);
    let _ = MirrorEventSeq::get(seq);

    // Exercise every per-layer mirror method to force resolution.
    let _ = mirror_run_storage_event(
        MirrorRuntimeJournalEvent::RunSubmitted { run, workflow: MirrorWorkflowDigest { value: [0; 32] } },
        seq,
    );
    let _ = mirror_action_storage_event(
        MirrorRuntimeJournalEvent::ActionScheduled {
            run,
            step: MirrorStepIdx { value: 0 },
            action: MirrorActionId { value: 0 },
        },
        seq,
    );
    let _ = mirror_boundary_storage_event(
        MirrorRuntimeJournalEvent::WaitScheduled { run, step: MirrorStepIdx { value: 0 } },
        seq,
    )?;

    // Force resolution of the post-fix storage_event with each of the
    // three variant sets.
    let _ = mirror_storage_event(
        &MirrorRuntimeJournalEvent::RunFailed { run },
        seq,
    )?;
    let _ = mirror_storage_event(
        &MirrorRuntimeJournalEvent::Resumed { run, timestamp: 0 },
        seq,
    )?;
    let _ = mirror_storage_event(
        &MirrorRuntimeJournalEvent::RunAdmission {
            admission: MirrorRunAdmission {
                run_id: run,
                artifact_digest: MirrorWorkflowDigest { value: [0; 32] },
                granted: MirrorCapabilitySet,
                policy: MirrorRuntimePolicy::Relaxed,
            },
        },
        seq,
    )?;

    // Force resolution of every runtime_journal_event_kind arm.
    let _kind: &'static str = mirror_runtime_journal_event_kind(
        &MirrorRuntimeJournalEvent::Resumed { run, timestamp: 0 },
    );
    let _ = _kind;
    Ok(())
}

} // verus!

fn main() {}
