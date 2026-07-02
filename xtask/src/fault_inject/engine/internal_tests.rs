#![forbid(unsafe_code)]

//! Internal unit tests for the fault injection engine. Canonical contract
//! checks live in `crates/workspace_tests/tests/fault_injection_tests.rs`.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use vb_core::ids::{ActionId, RunId, StepIdx};

    use crate::fault_inject::engine::{run_fault_injection, validate_config};
    use crate::fault_inject::report::FaultOutcome;
    use crate::fault_inject::types::{
        BoundarySlot, CheckpointSeq, CrashSeverity, FailureCode, FaultConfig, FaultError,
        FaultEvent, NamedBoundary,
    };

    fn boundary_runtime_before(run: u64, step: u16) -> NamedBoundary {
        NamedBoundary::RuntimeBeforeAppend {
            run: RunId::new(run),
            step: StepIdx::new(step),
        }
    }

    #[test]
    fn empty_schedule_returns_clean_report() {
        let config = FaultConfig::new(1, Vec::new(), Vec::new());
        let report = run_fault_injection(config).expect("empty schedule");
        assert_eq!(report.events_applied, 0);
        assert_eq!(report.runtime_steps, 0);
        assert!(!report.recovery_required);
        assert!(report.journal_entries.is_empty());
        assert!(report.outcomes.is_empty());
    }

    #[test]
    fn restart_event_requires_recovery() {
        let restart = NamedBoundary::Restart {
            checkpoint: CheckpointSeq(7),
        };
        let config = FaultConfig::new(
            2,
            vec![restart],
            vec![FaultEvent::Restart {
                checkpoint: CheckpointSeq(7),
            }],
        );
        let report = run_fault_injection(config).expect("run");
        assert!(report.recovery_required);
        assert!(report.journal_entries.is_empty());
        let _ = boundary_runtime_before;
    }

    #[test]
    fn action_failure_does_not_write_journal() {
        let action = ActionId::new(3);
        let config = FaultConfig::new(
            4,
            Vec::new(),
            vec![FaultEvent::ActionFailure {
                action,
                code: FailureCode::Network,
            }],
        );
        let report = run_fault_injection(config).expect("run");
        assert!(report.journal_entries.is_empty());
        assert!(!report.recovery_required);
        match &report.outcomes[0] {
            FaultOutcome::ActionFailed { action: a, code: c } => {
                assert_eq!(*a, action);
                assert_eq!(*c, FailureCode::Network);
            }
            _ => panic!("expected ActionFailed"),
        }
    }

    #[test]
    fn schedule_hash_changes_with_seed() {
        let boundary = boundary_runtime_before(9, 0);
        let cfg = |seed: u64| {
            FaultConfig::new(
                seed,
                vec![boundary.clone()],
                vec![FaultEvent::LockContention {
                    boundary: boundary.clone(),
                    retry_count: 0,
                }],
            )
        };
        let a = run_fault_injection(cfg(1)).expect("a");
        let b = run_fault_injection(cfg(2)).expect("b");
        assert_ne!(
            a.schedule_hash, b.schedule_hash,
            "different seeds must yield different hashes"
        );
    }

    #[test]
    fn boundary_slot_label_uses_before_after() {
        let before = NamedBoundary::ActionAction {
            action: ActionId::new(2),
            slot: BoundarySlot::Before,
        };
        let after = NamedBoundary::ActionAction {
            action: ActionId::new(2),
            slot: BoundarySlot::After,
        };
        assert_eq!(before.label(), "action/action2/before");
        assert_eq!(after.label(), "action/action2/after");
    }

    #[test]
    fn validate_config_rejects_zero_fault_budget() {
        let config = FaultConfig {
            seed: 0,
            boundaries: Vec::new(),
            fault_schedule: Vec::new(),
            max_faults: 0,
            max_runtime_steps: 1,
        };
        match validate_config(&config) {
            Err(FaultError::InvalidConfig(msg)) => assert!(msg.contains("max_faults")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn validate_config_rejects_unknown_boundary() {
        let known = boundary_runtime_before(1, 0);
        let unknown = NamedBoundary::StorageAppendCommit { partition: 99 };
        let config = FaultConfig::new(
            5,
            vec![known],
            vec![FaultEvent::Crash {
                boundary: unknown,
                severity: CrashSeverity::HardKill,
            }],
        );
        match validate_config(&config) {
            Err(FaultError::InvalidConfig(_)) => {}
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }
}
