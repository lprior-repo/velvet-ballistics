#![forbid(unsafe_code)]

//! Deterministic simulation engine for the fault injection model.

mod internal_tests;
mod state;

use std::collections::BTreeSet;

use state::SimulationState;

use crate::fault_inject::prng::compute_schedule_hash;
use crate::fault_inject::report::{FaultOutcome, FaultReport, JournalOutcome, MissingReason};
use crate::fault_inject::types::{
    CrashSeverity, FaultConfig, FaultError, FaultEvent, NamedBoundary,
};

/// Run the deterministic fault injection simulation.
///
/// # Errors
/// - [`FaultError::InvalidConfig`] for malformed config (zero budgets,
///   schedule referencing unknown boundaries, schedule overflow).
/// - [`FaultError::BudgetExceeded`] when applying the next fault would
///   violate `max_faults` or `max_runtime_steps`.
pub fn run_fault_injection(config: FaultConfig) -> Result<FaultReport, FaultError> {
    validate_config(&config)?;
    let boundary_set: BTreeSet<&NamedBoundary> = config.boundaries.iter().collect();
    let mut state = SimulationState::new(config.seed);
    for fault in &config.fault_schedule {
        apply_fault(
            fault,
            &boundary_set,
            &mut state,
            config.max_faults,
            config.max_runtime_steps,
        )?;
    }
    Ok(finalize_report(state))
}

/// Validate `FaultConfig` without running the simulation.
///
/// # Errors
/// - [`FaultError::InvalidConfig`] for zero budgets, schedule overflow,
///   or schedule referencing unknown boundaries.
pub fn validate_config(config: &FaultConfig) -> Result<(), FaultError> {
    if config.max_faults == 0 {
        return Err(FaultError::InvalidConfig(
            "max_faults must be > 0".to_owned(),
        ));
    }
    if config.max_runtime_steps == 0 {
        return Err(FaultError::InvalidConfig(
            "max_runtime_steps must be > 0".to_owned(),
        ));
    }
    // Bound the schedule to a sane upper limit so the engine never has to
    // walk an attacker-controlled schedule of unbounded length. On any
    // platform Rust targets today `u32` fits in `usize`, so this always
    // returns `Ok(u32::MAX)`. The fallback `usize::MAX` is purely
    // defensive for exotic future platforms.
    let max_len = usize::try_from(u32::MAX).unwrap_or(usize::MAX);
    if config.fault_schedule.len() > max_len {
        return Err(FaultError::InvalidConfig(
            "fault_schedule length exceeds u32::MAX".to_owned(),
        ));
    }
    let boundary_set: BTreeSet<&NamedBoundary> = config.boundaries.iter().collect();
    for (idx, fault) in config.fault_schedule.iter().enumerate() {
        if let Some(boundary) = NamedBoundary::for_fault(fault)
            && !boundary_set.contains(&boundary)
        {
            return Err(FaultError::InvalidConfig(format!(
                "fault_schedule[{idx}] references unknown boundary: {}",
                boundary.label()
            )));
        }
    }
    Ok(())
}

fn apply_fault(
    fault: &FaultEvent,
    boundary_set: &BTreeSet<&NamedBoundary>,
    state: &mut SimulationState,
    max_faults: u32,
    max_runtime_steps: u32,
) -> Result<(), FaultError> {
    state.consume_fault(max_faults)?;
    // Each fault costs at least one runtime step to model the boundary
    // passage overhead.
    state.consume_steps(1, max_runtime_steps)?;

    match fault {
        FaultEvent::Crash { boundary, severity } => {
            apply_crash(boundary.clone(), *severity, state)?;
        }
        FaultEvent::AppendFailure {
            boundary,
            transient,
        } => {
            apply_append_failure(boundary.clone(), *transient, state, max_runtime_steps)?;
        }
        FaultEvent::LockContention {
            boundary,
            retry_count,
        } => {
            apply_lock_contention(boundary.clone(), *retry_count, state, max_runtime_steps)?;
        }
        FaultEvent::ActionFailure { action, code } => {
            state.outcomes.push(FaultOutcome::ActionFailed {
                action: *action,
                code: *code,
            });
        }
        FaultEvent::Timeout { step, delay_ticks } => {
            // A timeout also costs `delay_ticks` runtime steps to model
            // the caller waiting out the timer.
            state.consume_steps(*delay_ticks, max_runtime_steps)?;
            state.outcomes.push(FaultOutcome::TimedOut {
                step: *step,
                delay_ticks: *delay_ticks,
            });
        }
        FaultEvent::Restart { checkpoint } => {
            state.outcomes.push(FaultOutcome::Restarted {
                checkpoint: *checkpoint,
            });
            state.recovery_required = true;
        }
    }

    // Silence unused warning for boundary_set in the body above; we keep
    // the binding so the borrow checker is happy across match arms.
    let _ = boundary_set;
    Ok(())
}

fn apply_crash(
    boundary: NamedBoundary,
    severity: CrashSeverity,
    state: &mut SimulationState,
) -> Result<(), FaultError> {
    state.outcomes.push(FaultOutcome::Crashed {
        boundary: boundary.clone(),
        severity,
    });
    let outcome = match &boundary {
        NamedBoundary::RuntimeBeforeAppend { .. } | NamedBoundary::StorageAppendStart { .. } => {
            JournalOutcome::Missing {
                boundary: boundary.clone(),
                reason: MissingReason::CrashBeforeAppend,
            }
        }
        NamedBoundary::RuntimeAfterAppend { .. }
        | NamedBoundary::StorageAppendMid { .. }
        | NamedBoundary::StorageAppendCommit { .. } => {
            let seq = state.alloc_seq()?;
            JournalOutcome::Appended {
                boundary: boundary.clone(),
                seq,
            }
        }
        _ => JournalOutcome::Pending {
            boundary: boundary.clone(),
        },
    };
    state.journal_entries.push(outcome);
    // A crash of any severity forces recovery.
    state.recovery_required = true;
    Ok(())
}

fn apply_append_failure(
    boundary: NamedBoundary,
    transient: bool,
    state: &mut SimulationState,
    max_runtime_steps: u32,
) -> Result<(), FaultError> {
    if transient {
        // Spend one extra step modelling the retry attempt.
        state.consume_steps(1, max_runtime_steps)?;
        let seq = state.alloc_seq()?;
        state.outcomes.push(FaultOutcome::AppendFailed {
            boundary: boundary.clone(),
            transient: true,
            attempts: 2,
        });
        state
            .journal_entries
            .push(JournalOutcome::Appended { boundary, seq });
        Ok(())
    } else {
        state.outcomes.push(FaultOutcome::AppendFailed {
            boundary: boundary.clone(),
            transient: false,
            attempts: 1,
        });
        state.journal_entries.push(JournalOutcome::Missing {
            boundary,
            reason: MissingReason::AppendFailurePermanent,
        });
        state.recovery_required = true;
        Ok(())
    }
}

fn apply_lock_contention(
    boundary: NamedBoundary,
    retry_count: u8,
    state: &mut SimulationState,
    max_runtime_steps: u32,
) -> Result<(), FaultError> {
    // We model lock contention as a bounded retry loop. Each retry costs
    // one runtime step. If retry_count is 0 we treat it as "use the seed
    // to choose" — this is the surface where the seed influences the
    // outcome, ensuring that different seeds produce different reports.
    let effective_retries: u8 = if retry_count == 0 {
        // PRNG picks 0..=8 retries; if 0 is chosen the first attempt
        // fails and the entry is missing.
        state.prng.next_u8_in_range(8)
    } else {
        retry_count.saturating_sub(1)
    };

    // Spend one step per retry.
    let retry_budget = u32::from(effective_retries);
    state.consume_steps(retry_budget, max_runtime_steps)?;

    // Simulate: each retry has a 50% chance of acquiring the lock based
    // on the next PRNG draw. This is the second surface where the seed
    // diverges outcomes across runs.
    let mut acquired = false;
    let mut attempts: u8 = 0;
    for _ in 0..=effective_retries {
        attempts = attempts.saturating_add(1);
        if state.prng.next_u8_in_range(1) == 0 {
            acquired = true;
            break;
        }
    }

    if acquired {
        let seq = state.alloc_seq()?;
        state.outcomes.push(FaultOutcome::LockResolved {
            boundary: boundary.clone(),
            attempts,
        });
        state
            .journal_entries
            .push(JournalOutcome::Appended { boundary, seq });
    } else {
        state.outcomes.push(FaultOutcome::LockExhausted {
            boundary: boundary.clone(),
            attempts,
        });
        state.journal_entries.push(JournalOutcome::Missing {
            boundary,
            reason: MissingReason::LockContentionExhausted,
        });
        state.recovery_required = true;
    }
    Ok(())
}

fn finalize_report(state: SimulationState) -> FaultReport {
    let schedule_hash = compute_schedule_hash(state.seed, &state.outcomes, &state.journal_entries);
    FaultReport {
        seed: state.seed,
        events_applied: state.events_applied,
        runtime_steps: state.runtime_steps,
        journal_entries: state.journal_entries,
        outcomes: state.outcomes,
        recovery_required: state.recovery_required,
        schedule_hash,
    }
}
