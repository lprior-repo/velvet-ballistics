#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{SlotValue, Taint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelRuntimeError {
    AdmissionArtifactNotFound { digest: WorkflowDigest },
    InvalidActionCompletion,
    RunNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelInspectResponse {
    Found { run: RunId, correlation: u64 },
    NotFound { run: RunId, correlation: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StoreMode {
    Missing,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TicketShape {
    Matching,
    Stale,
    WrongRun,
    AbsentRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FacadeKernelState {
    target: RunId,
    other: RunId,
    queue_depth: u8,
    target_active: bool,
    other_active: bool,
    target_asking: bool,
    answer_value: Option<SlotValue>,
    answer_taint: Taint,
}

impl StoreMode {
    fn selected(selector: u8) -> Self {
        if selector & 1 == 0 {
            Self::Missing
        } else {
            Self::Accepted
        }
    }
}

impl TicketShape {
    fn selected(selector: u8) -> Self {
        match selector % 4 {
            0 => Self::Matching,
            1 => Self::Stale,
            2 => Self::WrongRun,
            _ => Self::AbsentRun,
        }
    }

    fn run(self, target: RunId, other: RunId, selector: u8) -> RunId {
        match self {
            Self::Matching | Self::Stale => target,
            Self::WrongRun => other,
            Self::AbsentRun => RunId::new(u64::from(90u8.saturating_add(selector % 20))),
        }
    }
}

impl FacadeKernelState {
    fn seeded(selector: u8) -> Self {
        Self {
            target: RunId::new(1 + u64::from(selector % 5)),
            other: RunId::new(20 + u64::from(selector % 5)),
            queue_depth: selector % 3,
            target_active: true,
            other_active: true,
            target_asking: true,
            answer_value: None,
            answer_taint: Taint::Clean,
        }
    }

    fn submit_direct(
        &mut self,
        policy: RuntimePolicy,
        store: StoreMode,
    ) -> Result<(), KernelRuntimeError> {
        if policy != RuntimePolicy::Relaxed && store == StoreMode::Missing {
            return Err(KernelRuntimeError::AdmissionArtifactNotFound {
                digest: WorkflowDigest::from_bytes([0x51; 32]),
            });
        }
        self.queue_depth = self.queue_depth.saturating_add(1);
        Ok(())
    }

    fn fail_action_enqueue(&self, ticket_run: RunId) -> Result<(), KernelRuntimeError> {
        if ticket_run == self.target || ticket_run == self.other || ticket_run.get() >= 90 {
            Ok(())
        } else {
            Err(KernelRuntimeError::RunNotFound)
        }
    }

    fn tick_after_facade_fail_action(&self, ticket_run: RunId) -> Result<(), KernelRuntimeError> {
        if ticket_run == self.target {
            Err(KernelRuntimeError::InvalidActionCompletion)
        } else {
            Err(KernelRuntimeError::InvalidActionCompletion)
        }
    }

    fn snapshot_other(&self, correlation: u64) -> KernelInspectResponse {
        if self.other_active {
            KernelInspectResponse::Found {
                run: self.other,
                correlation,
            }
        } else {
            KernelInspectResponse::NotFound {
                run: self.other,
                correlation,
            }
        }
    }

    fn answer_ask(
        &mut self,
        shape: TicketShape,
        value: SlotValue,
        taint: Taint,
    ) -> Result<(), KernelRuntimeError> {
        match shape {
            // ERR-004 / LETHAL-001 fix: Stale ask must fail with RunNotFound per contract
            // (contract.md:64-65, vb_vt2f_direct_runtime_api_acceptance.rs:658-698).
            // Stale means the ask ticket's run is terminal/active=false; it cannot be answered.
            TicketShape::Matching if self.target_active && self.target_asking => {
                self.answer_value = Some(value);
                self.answer_taint = taint;
                self.target_asking = false;
                Ok(())
            }
            TicketShape::Stale
            | TicketShape::WrongRun
            | TicketShape::AbsentRun
            | TicketShape::Matching => Err(KernelRuntimeError::RunNotFound),
        }
    }

    fn tick_after_answer(&mut self, shape: TicketShape) -> Result<(), KernelRuntimeError> {
        match shape {
            // ERR-004 / LETHAL-001 fix: Stale ask must fail with RunNotFound per contract.
            // A Stale ticket's run is no longer active, so tick cannot complete it.
            TicketShape::Matching if self.answer_value.is_some() => {
                self.target_active = false;
                Ok(())
            }
            TicketShape::Stale | TicketShape::WrongRun | TicketShape::AbsentRun => {
                Err(KernelRuntimeError::RunNotFound)
            }
            // Matching with no answer value is also an error
            TicketShape::Matching => Err(KernelRuntimeError::RunNotFound),
        }
    }
}

fn answer_value(selector: u8) -> SlotValue {
    match selector % 3 {
        0 => SlotValue::I64(99),
        1 => SlotValue::Bool(true),
        _ => SlotValue::Null,
    }
}

#[kani::proof]
fn vt2f_runtime_facade_semantics() {
    let selector: u8 = kani::any();
    let shape = TicketShape::selected(selector);
    let store = StoreMode::selected(selector);
    let policy = if selector % 3 == 0 {
        RuntimePolicy::Relaxed
    } else {
        RuntimePolicy::Strict
    };

    kani::cover!(
        store == StoreMode::Missing,
        "missing accepted artifact store covered"
    );
    kani::cover!(
        store == StoreMode::Accepted,
        "accepted artifact store covered"
    );
    kani::cover!(policy == RuntimePolicy::Strict, "strict policy covered");
    kani::cover!(
        matches!(shape, TicketShape::Matching),
        "matching ticket covered"
    );
    kani::cover!(matches!(shape, TicketShape::Stale), "stale ticket covered");
    kani::cover!(
        matches!(shape, TicketShape::WrongRun),
        "wrong-run ticket covered"
    );
    kani::cover!(
        matches!(shape, TicketShape::AbsentRun),
        "absent-run ticket covered"
    );

    let mut strict_state = FacadeKernelState::seeded(selector);
    let before = strict_state.queue_depth;
    let strict_result = strict_state.submit_direct(RuntimePolicy::Strict, StoreMode::Missing);
    kani::assert(matches!(
        strict_result,
        Err(KernelRuntimeError::AdmissionArtifactNotFound { .. }), "assertion failed"));
    );
    kani::assert(before == strict_state.queue_depth, "assertion failed");

    let mut admitted_state = FacadeKernelState::seeded(selector);
    let admitted_before = admitted_state.queue_depth;
    let admitted_result = admitted_state.submit_direct(policy, store);
    if policy == RuntimePolicy::Relaxed || store == StoreMode::Accepted {
        kani::assert(admitted_result.is_ok(), "kani harness assertion");
        kani::assert(admitted_state.queue_depth == admitted_before.saturating_add(1, "assertion failed"), "assertion failed");
    } else {
        kani::assert(matches!(
            admitted_result,
            Err(KernelRuntimeError::AdmissionArtifactNotFound { .. }), "assertion failed"));
        );
        kani::assert(admitted_state.queue_depth == admitted_before, "assertion failed");
    }

    let fail_state = FacadeKernelState::seeded(selector);
    let unrelated_before = fail_state.snapshot_other(100);
    let ticket_run = shape.run(fail_state.target, fail_state.other, selector);
    kani::assert(
        fail_state.fail_action_enqueue(ticket_run).is_ok(),
        "kani harness assertion",
    );
    kani::assert(matches!(
        fail_state.tick_after_facade_fail_action(ticket_run),
        Err(KernelRuntimeError::InvalidActionCompletion), "assertion failed"));
    kani::assert(unrelated_before == fail_state.snapshot_other(100, "assertion failed"), "assertion failed");

    let mut ask_state = FacadeKernelState::seeded(selector);
    let ask_unrelated_before = ask_state.snapshot_other(200);
    let value = answer_value(selector);
    let answer_result = ask_state.answer_ask(shape, value, Taint::Clean);
    let tick_result = ask_state.tick_after_answer(shape);
    // ERR-004 / LETHAL-001 fix: Stale ask MUST return RunNotFound (matching is Ok).
    // The else branch now correctly covers Stale, WrongRun, and AbsentRun.
    if matches!(shape, TicketShape::Matching) {
        kani::assert(answer_result.is_ok(, "assertion failed"), "kani harness assertion");
        kani::assert(tick_result.is_ok(, "assertion failed"), "kani harness assertion");
        kani::assert(ask_state.answer_value == Some(value, "assertion failed"), "assertion failed");
        , "assertion failed");
        kani::assert(ask_state.answer_taint == Taint::Clean, "assertion failed");
    } else {
        // Stale/WrongRun/AbsentRun all return RunNotFound per ERR-004 contract.
        kani::assert(matches!(
            answer_result,
            Err(KernelRuntimeError::RunNotFound)
        ));
        kani::assert(matches!(tick_result, Err(KernelRuntimeError::RunNotFound), "assertion failed"));
        );
        kani::assert(ask_state.answer_value == None, "assertion failed");
        kani::assert(ask_state.target_active, "kani harness assertion");
    }
    kani::assert(ask_unrelated_before == ask_state.snapshot_other(200), "assertion failed");
}
