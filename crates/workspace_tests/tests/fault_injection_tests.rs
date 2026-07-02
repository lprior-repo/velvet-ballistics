#![forbid(unsafe_code)]

//! vb-wy33p.12 — deterministic runtime/journal fault injection engine tests.
//!
//! These tests exercise the public contract of `xtask::fault_inject`:
//!
//! 1. `fault_injection_determinism` — same seed + config → identical
//!    `FaultReport`.
//! 2. `fault_injection_respects_max_faults` — exceeding `max_faults`
//!    returns `FaultError::BudgetExceeded`, no panic.
//! 3. `fault_injection_crash_before_append` — journal shows the event
//!    missing; recovery is required.
//! 4. `fault_injection_crash_after_append` — journal shows the event
//!    present; no recovery required from the journal perspective.
//! 5. `fault_injection_lock_contention_retries_then_succeeds` — a
//!    transient lock contention resolves within the retry budget.
//! 6. `fault_injection_seeded_faults_explode_in_different_orders` —
//!    different seeds produce different fault outcome orderings.
//!
//! The tests are pure (no IO, no threads) and follow the strict source
//! lint policy: `unwrap`/`expect`/`panic` are used only inside `#[test]`
//! functions (test targets are exempt from the production strict lint).

use vb_core::ids::{ActionId, RunId, StepIdx};
use xtask::fault_inject::{
    BoundarySlot, BudgetKind, CheckpointSeq, CrashSeverity, FailureCode, FaultConfig, FaultError,
    FaultEvent, FaultOutcome, FaultReport, JournalOutcome, MissingReason, NamedBoundary,
    run_fault_injection,
};

// ---------------------------------------------------------------------------
// Helpers (no panic paths, pure).
// ---------------------------------------------------------------------------

fn boundary_runtime_before(run: u64, step: u16) -> NamedBoundary {
    NamedBoundary::RuntimeBeforeAppend {
        run: RunId::new(run),
        step: StepIdx::new(step),
    }
}

fn boundary_runtime_after(run: u64, step: u16) -> NamedBoundary {
    NamedBoundary::RuntimeAfterAppend {
        run: RunId::new(run),
        step: StepIdx::new(step),
    }
}

fn boundary_storage_start(partition: u8) -> NamedBoundary {
    NamedBoundary::StorageAppendStart { partition }
}

fn config_with_schedule(
    seed: u64,
    boundaries: Vec<NamedBoundary>,
    schedule: Vec<FaultEvent>,
) -> FaultConfig {
    FaultConfig::new(seed, boundaries, schedule)
}

// ---------------------------------------------------------------------------
// 1. Determinism — same seed + same config → identical report.
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_determinism() {
    let boundary = boundary_runtime_before(1, 0);
    let restart_boundary = NamedBoundary::Restart {
        checkpoint: CheckpointSeq(3),
    };
    let schedule = vec![
        FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::HardKill,
        },
        FaultEvent::Restart {
            checkpoint: CheckpointSeq(3),
        },
    ];
    let build = || {
        config_with_schedule(
            0xCAFE_F00D_DEAD_BEEF,
            vec![boundary.clone(), restart_boundary.clone()],
            schedule.clone(),
        )
    };
    let a: FaultReport = run_fault_injection(build()).expect("first run");
    let b: FaultReport = run_fault_injection(build()).expect("second run");
    assert_eq!(
        a, b,
        "same seed and config must produce identical FaultReport (determinism contract)"
    );
    assert_eq!(
        a.schedule_hash, b.schedule_hash,
        "schedule_hash must be deterministic for identical inputs"
    );
}

// ---------------------------------------------------------------------------
// 2. Max-faults budget — exceeding the limit returns typed error.
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_respects_max_faults() {
    let boundary = boundary_runtime_before(7, 3);
    // 5 faults in the schedule but budget only allows 2.
    let schedule = vec![
        FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::SoftPanic,
        },
        FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::HardKill,
        },
        FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::SoftPanic,
        },
        FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::HardKill,
        },
        FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::SoftPanic,
        },
    ];
    let config = config_with_schedule(123, vec![boundary.clone()], schedule)
        .with_max_faults(2)
        .with_max_runtime_steps(1024);
    match run_fault_injection(config) {
        Err(FaultError::BudgetExceeded {
            budget_kind: BudgetKind::Faults,
            observed,
            limit,
        }) => {
            assert_eq!(observed, 3, "third fault must be the one that exceeded");
            assert_eq!(limit, 2);
        }
        Err(other) => panic!("expected Faults budget exceeded, got {other:?}"),
        Ok(report) => panic!("expected error but got report: {report:?}"),
    }
}

// ---------------------------------------------------------------------------
// 3. Crash before append — journal entry missing, recovery required.
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_crash_before_append() {
    let boundary = boundary_runtime_before(11, 4);
    let config = config_with_schedule(
        9,
        vec![boundary.clone()],
        vec![FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::HardKill,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    assert_eq!(report.events_applied, 1, "exactly one fault applied");
    assert!(
        report.recovery_required,
        "crash before append must trigger recovery"
    );
    assert_eq!(report.journal_entries.len(), 1);
    match &report.journal_entries[0] {
        JournalOutcome::Missing {
            boundary: b,
            reason,
        } => {
            assert_eq!(b, &boundary);
            assert_eq!(*reason, MissingReason::CrashBeforeAppend);
        }
        other => panic!("expected Missing(CrashBeforeAppend), got {other:?}"),
    }
    match &report.outcomes[0] {
        FaultOutcome::Crashed {
            boundary: b,
            severity,
        } => {
            assert_eq!(b, &boundary);
            assert_eq!(*severity, CrashSeverity::HardKill);
        }
        other => panic!("expected Crashed outcome, got {other:?}"),
    }
}

#[test]
fn fault_injection_crash_before_append_storage_start() {
    let boundary = boundary_storage_start(2);
    let config = config_with_schedule(
        10,
        vec![boundary.clone()],
        vec![FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::SoftPanic,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    assert!(report.recovery_required);
    match &report.journal_entries[0] {
        JournalOutcome::Missing {
            boundary: b,
            reason,
        } => {
            assert_eq!(b, &boundary);
            assert_eq!(*reason, MissingReason::CrashBeforeAppend);
        }
        other => panic!("expected Missing, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 4. Crash after append — journal entry present, recovery not required.
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_crash_after_append() {
    let boundary = boundary_runtime_after(11, 4);
    let config = config_with_schedule(
        11,
        vec![boundary.clone()],
        vec![FaultEvent::Crash {
            boundary: boundary.clone(),
            severity: CrashSeverity::SoftPanic,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    assert_eq!(report.events_applied, 1);
    assert_eq!(report.journal_entries.len(), 1);
    match &report.journal_entries[0] {
        JournalOutcome::Appended { boundary: b, seq } => {
            assert_eq!(b, &boundary);
            assert_eq!(*seq, 1, "first appended event must have seq=1");
        }
        other => panic!("expected Appended, got {other:?}"),
    }
    // The crash still requires recovery for the unfinished step, but the
    // journal entry is durable — the runtime can re-derive state from the
    // committed event without re-applying it.
    assert!(
        report.recovery_required,
        "soft panic after append still flags recovery for the unfinished step"
    );
}

#[test]
fn fault_injection_append_failure_transient_recovers() {
    let boundary = boundary_runtime_after(20, 1);
    let config = config_with_schedule(
        20,
        vec![boundary.clone()],
        vec![FaultEvent::AppendFailure {
            boundary: boundary.clone(),
            transient: true,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    assert!(!report.recovery_required, "transient retry must succeed");
    match &report.journal_entries[0] {
        JournalOutcome::Appended { boundary: b, seq } => {
            assert_eq!(b, &boundary);
            assert_eq!(*seq, 1);
        }
        other => panic!("expected Appended, got {other:?}"),
    }
    match &report.outcomes[0] {
        FaultOutcome::AppendFailed {
            boundary: b,
            transient,
            attempts,
        } => {
            assert_eq!(b, &boundary);
            assert!(*transient);
            assert_eq!(*attempts, 2, "transient failure must record two attempts");
        }
        other => panic!("expected AppendFailed outcome, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Lock contention retries then succeeds (recovery from transient lock).
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_lock_contention_retries_then_succeeds() {
    let boundary = boundary_runtime_before(33, 0);
    // Pin the schedule to a known seed + retry_count = 8 (well above the
    // PRNG's 0..=8 cap), so the retry loop is guaranteed to drive the
    // lock to acquisition.
    let config = config_with_schedule(
        0xDEAD_BEEF_CAFE_F00D,
        vec![boundary.clone()],
        vec![FaultEvent::LockContention {
            boundary: boundary.clone(),
            retry_count: 8,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    assert!(
        !report.recovery_required,
        "lock resolved within retry budget must not require recovery"
    );
    match &report.journal_entries[0] {
        JournalOutcome::Appended { boundary: b, seq } => {
            assert_eq!(b, &boundary);
            assert_eq!(*seq, 1);
        }
        other => panic!("expected Appended after lock resolution, got {other:?}"),
    }
    match &report.outcomes[0] {
        FaultOutcome::LockResolved {
            boundary: b,
            attempts,
        } => {
            assert_eq!(b, &boundary);
            assert!(*attempts >= 1, "lock must record at least one attempt");
            assert!(*attempts <= 8, "lock must respect retry_count upper bound");
        }
        other => panic!("expected LockResolved, got {other:?}"),
    }
}

#[test]
fn fault_injection_lock_contention_exhaustion_requires_recovery() {
    // With retry_count = 1 and a deterministic seed, the single retry
    // attempt may either succeed or fail. We probe a range of seeds and
    // assert that *at least one* seed yields a LockExhausted outcome —
    // this proves the engine correctly emits both branches.
    let boundary = boundary_runtime_before(34, 0);
    let mut found_exhausted = false;
    for seed in 0..64u64 {
        let config = config_with_schedule(
            seed,
            vec![boundary.clone()],
            vec![FaultEvent::LockContention {
                boundary: boundary.clone(),
                retry_count: 1,
            }],
        );
        let report = run_fault_injection(config).expect("run");
        if matches!(report.outcomes[0], FaultOutcome::LockExhausted { .. }) {
            found_exhausted = true;
            assert!(
                report.recovery_required,
                "LockExhausted must flag recovery_required"
            );
            match &report.journal_entries[0] {
                JournalOutcome::Missing {
                    reason: MissingReason::LockContentionExhausted,
                    ..
                } => {}
                other => panic!("expected Missing(LockContentionExhausted), got {other:?}"),
            }
            break;
        }
    }
    assert!(
        found_exhausted,
        "at least one seed must produce LockExhausted to prove the branch"
    );
}

// ---------------------------------------------------------------------------
// 6. Seed divergence — different seeds produce different report orderings.
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_seeded_faults_explode_in_different_orders() {
    // We schedule a LockContention with retry_count = 0 so the engine
    // falls back to the PRNG. Different seeds must produce different
    // (attempts, journal) combinations.
    let boundary = boundary_runtime_before(50, 0);
    let mut observed_attempts: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
    let mut observed_journal_kinds: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut observed_hashes: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    let mut observed_outcomes: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();

    for seed in 0..32u64 {
        let config = config_with_schedule(
            seed,
            vec![boundary.clone()],
            vec![FaultEvent::LockContention {
                boundary: boundary.clone(),
                retry_count: 0,
            }],
        );
        let report = run_fault_injection(config).expect("run");
        match &report.outcomes[0] {
            FaultOutcome::LockResolved { attempts, .. } => {
                observed_attempts.insert(*attempts);
            }
            FaultOutcome::LockExhausted { attempts, .. } => {
                observed_attempts.insert(*attempts);
            }
            other => panic!("unexpected outcome kind: {other:?}"),
        }
        observed_hashes.insert(report.schedule_hash);
        let journal_kind = match &report.journal_entries[0] {
            JournalOutcome::Appended { .. } => "appended".to_owned(),
            JournalOutcome::Missing { reason, .. } => format!("missing:{reason:?}"),
            JournalOutcome::Pending { .. } => "pending".to_owned(),
            JournalOutcome::Corrupt { .. } => "corrupt".to_owned(),
        };
        observed_journal_kinds.insert(journal_kind);
        let outcome_kind = match &report.outcomes[0] {
            FaultOutcome::LockResolved { .. } => "resolved".to_owned(),
            FaultOutcome::LockExhausted { .. } => "exhausted".to_owned(),
            _ => unreachable!(),
        };
        observed_outcomes.insert(outcome_kind);
    }

    assert!(
        observed_journal_kinds.len() >= 2,
        "different seeds must produce a mix of journal outcomes; got {observed_journal_kinds:?}"
    );
    assert!(
        observed_outcomes.len() >= 2,
        "different seeds must produce a mix of resolved/exhausted outcomes; got {observed_outcomes:?}"
    );
    assert!(
        observed_hashes.len() >= 4,
        "schedule_hash must diverge across seeds; got {} distinct values",
        observed_hashes.len()
    );
}

// ---------------------------------------------------------------------------
// Auxiliary contract checks (do not count toward the 6 required tests but
// reinforce the engine contract).
// ---------------------------------------------------------------------------

#[test]
fn fault_injection_action_failure_records_outcome_without_journal() {
    let action = ActionId::new(7);
    let boundary = boundary_runtime_before(70, 0);
    let config = config_with_schedule(
        70,
        vec![boundary.clone()],
        vec![FaultEvent::ActionFailure {
            action,
            code: FailureCode::Network,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    assert_eq!(report.events_applied, 1);
    assert!(report.journal_entries.is_empty());
    assert!(!report.recovery_required);
    match &report.outcomes[0] {
        FaultOutcome::ActionFailed { action: a, code } => {
            assert_eq!(*a, action);
            assert_eq!(*code, FailureCode::Network);
        }
        other => panic!("expected ActionFailed, got {other:?}"),
    }
}

#[test]
fn fault_injection_timeout_consumes_runtime_steps() {
    let step = StepIdx::new(9);
    let config = config_with_schedule(
        71,
        Vec::new(),
        vec![FaultEvent::Timeout {
            step,
            delay_ticks: 5,
        }],
    );
    let report = run_fault_injection(config).expect("run");
    // Timeout consumes 1 boundary step + 5 delay_ticks = 6 total.
    assert_eq!(report.runtime_steps, 6);
    assert!(!report.recovery_required);
    match &report.outcomes[0] {
        FaultOutcome::TimedOut {
            step: s,
            delay_ticks,
        } => {
            assert_eq!(*s, step);
            assert_eq!(*delay_ticks, 5);
        }
        other => panic!("expected TimedOut, got {other:?}"),
    }
}

#[test]
fn fault_injection_zero_budget_is_invalid_config() {
    let config = FaultConfig {
        seed: 0,
        boundaries: Vec::new(),
        fault_schedule: Vec::new(),
        max_faults: 0,
        max_runtime_steps: 1,
    };
    match run_fault_injection(config) {
        Err(FaultError::InvalidConfig(msg)) => {
            assert!(
                msg.contains("max_faults"),
                "msg should mention max_faults: {msg}"
            );
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn fault_injection_unknown_boundary_is_invalid_config() {
    let known = boundary_runtime_before(1, 0);
    let unknown = NamedBoundary::StorageAppendCommit { partition: 99 };
    let config = config_with_schedule(
        72,
        vec![known],
        vec![FaultEvent::Crash {
            boundary: unknown,
            severity: CrashSeverity::HardKill,
        }],
    );
    match run_fault_injection(config) {
        Err(FaultError::InvalidConfig(_)) => {}
        other => panic!("expected InvalidConfig for unknown boundary, got {other:?}"),
    }
}

#[test]
fn fault_injection_label_is_stable() {
    let boundary = NamedBoundary::ActionAction {
        action: ActionId::new(3),
        slot: BoundarySlot::Before,
    };
    assert_eq!(
        boundary.label(),
        "action/action3/before",
        "label must be stable across runs"
    );
}
