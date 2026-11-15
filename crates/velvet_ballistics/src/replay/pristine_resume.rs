use crate::app_impl::prelude::*;
use crate::run_compiled_runtime::completion_output::completion_after_execution;
use vb_runtime::runtime::completion_observation::{
    CompletionObservation, CompletionObservationSlot,
};
#[path = "pristine_resume/command.rs"]
mod command;
pub(super) use command::run_pristine_resume;

pub(super) fn pristine_resume_candidate(error: &vb_core::errors::CoreError) -> bool {
    matches!(
        error,
        vb_core::errors::CoreError::LifecycleDuplicateRequest {
            command: Some("resume"),
            ..
        } | vb_core::errors::CoreError::LifecycleInvalidTransition {
            command: Some("resume"),
            ..
        }
    )
}

pub(super) fn select_pristine_resume(
    run: vb_core::RunId,
    original: vb_core::errors::CoreError,
    events: Result<Vec<vb_storage::JournalEvent>, vb_storage::JournalError>,
) -> Result<(), vb_core::errors::CoreError> {
    use vb_storage::{JournalEvent, RunInputBinding, RunPolicyPin};
    if pristine_resume_candidate(&original)
        && let Ok(events) = events
        && let [
            JournalEvent::RunAccepted {
                run: accepted,
                seq: first,
                ..
            },
            JournalEvent::RunAdmissionBudget {
                run: admitted,
                seq: second,
                policy: vb_core::RuntimePolicy::Strict,
                input_digest: RunInputBinding::Digest(_),
                policy_pin: RunPolicyPin::Pinned(_),
                ..
            },
            tail @ ..,
        ] = events.as_slice()
        && *accepted == run
        && *admitted == run
        && first.get() == 0
        && second.get() == 1
        && pristine_tail(run, tail)
    {
        return Ok(());
    }
    Err(original)
}

fn pristine_tail(run: vb_core::RunId, events: &[vb_storage::JournalEvent]) -> bool {
    match events {
        [
            vb_storage::JournalEvent::CompatibilityDecisionEvent {
                run: owner,
                seq,
                decision,
            },
            vb_storage::JournalEvent::RunBudgetOriginEvent {
                run: origin_owner,
                seq: origin_seq,
                started_millis,
            },
        ] => {
            *owner == run
                && seq.get() == 2
                && *origin_owner == run
                && origin_seq.get() == 3
                && *started_millis > 0
                && decision.boundary == vb_storage::compat::CompatBoundary::IrVersion
                && decision.authorize_live_effect().is_ok()
        }
        _ => false,
    }
}

fn recover_pristine_once(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: vb_core::RunId,
) -> Result<Option<CompletionObservation>, String> {
    use vb_runtime::recovery::{RecoveredRunBoundaryKind, RuntimeRecoveryProduct};
    match runtime
        .recover_product(run)
        .map_err(|error| format!("recovery: {error}"))?
    {
        RuntimeRecoveryProduct::Resumable(product)
            if product.boundary_kind() == RecoveredRunBoundaryKind::None => {}
        other => {
            return Err(format!(
                "pristine recovery requires a boundary-free product: {other:?}"
            ));
        }
    }
    runtime
        .recover_and_resume(run)
        .map_err(|error| format!("recovery activation: {error}"))?;
    let mut slots = [CompletionObservationSlot::default()];
    let observed = runtime
        .tick_all_observed(&mut slots)
        .map_err(|error| format!("observation preflight: {error:?}"))?;
    let [slot] = slots;
    completion_after_execution(observed, slot).map_err(|error| format!("runtime tick: {error}"))
}

fn retain_resume_failure(failure: &mut Option<String>, detail: String) {
    match failure {
        Some(primary) => {
            primary.push_str("; cleanup: ");
            primary.push_str(&detail);
        }
        None => *failure = Some(detail),
    }
}

fn finalize_pristine_resume(
    mut runtime: vb_runtime::runtime::Runtime,
    journal: Arc<vb_storage::FjallJournal>,
    completed: bool,
    failure: &mut Option<String>,
) {
    let pending = runtime.pending_boundary_snapshot(1);
    if pending.command_queue_depth() != 0 {
        retain_resume_failure(
            failure,
            "queued commands remain; shutdown was not driven".into(),
        );
    } else {
        match runtime.shutdown_graceful() {
            Ok(report) => {
                let remaining = report.remaining();
                if report.commands_drained() != 0
                    || remaining.command_queue_depth() != 0
                    || remaining.active_run_count() != pending.active_run_count()
                    || remaining.pending_ask_count() != pending.pending_ask_count()
                    || remaining.pending_action_count() != pending.pending_action_count()
                    || remaining.pending_timer_count() != pending.pending_timer_count()
                    || (completed
                        && (remaining.active_run_count() != 0
                            || remaining.pending_ask_count() != 0
                            || remaining.pending_action_count() != 0
                            || remaining.pending_timer_count() != 0))
                    || (remaining.pending_ask_count() == 1 && remaining.active_run_count() != 1)
                {
                    retain_resume_failure(
                        failure,
                        format!("unexpected shutdown report: {report:?}"),
                    );
                }
            }
            Err(error) => retain_resume_failure(failure, format!("shutdown: {error:?}")),
        }
    }
    drop(runtime);
    match Arc::try_unwrap(journal) {
        Ok(mut owned) => {
            if let Err(error) = owned.close() {
                retain_resume_failure(failure, format!("journal close: {error}"));
            }
            drop(owned);
        }
        Err(shared) => {
            retain_resume_failure(failure, "journal owner survived runtime shutdown".into());
            drop(shared);
        }
    }
}

#[cfg(test)]
#[path = "pristine_resume_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "pristine_resume_compat_tests.rs"]
mod compatibility_tests;
