    use super::*;
    use crate::shard::{RuntimeState, ShardConfig};
    use vb_storage::EventSeq;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn test_shard() -> TestResult<Shard> {
        Ok(Shard::new(ShardConfig {
            command_queue_capacity: 64,
            trace_capacity: 64,
            step_budget_per_tick: 4,
            max_active_runs: 8,
            max_open_gates: 8,
            policy: vb_core::RuntimePolicy::Relaxed,
        })?)
    }

    // Direct cleanup tests with synthetic aggregate ownership. These do not
    // inject allocation failure, execute a drive, or establish recovery safety.
    #[test]
    fn failed_submit_cleanup_retains_checked_out_and_runtime_owned_accounting() -> TestResult {
        for runtime_only in [false, true] {
            let mut shard = test_shard()?;
            let run = RunId::new(72_101);
            shard.checked_out_run_insert(run)?;
            if runtime_only {
                shard.runtime_state_insert(run, RuntimeState::Running)?;
                shard.checked_out_run_remove(run);
            }
            let tenant = Some(vb_core::ids::TenantId::new(93));
            shard.charge_admission_quota(run, tenant, 7)?;
            shard.seed_journal_sequence(run, EventSeq::new(100))?;
            let quota = shard.charged_quotas.clone();
            let usage = shard.usage_snapshot();
            let runtime_state = shard.runtime_state_get(run);
            let checked_out = shard.checked_out_run_contains(run);
            for _ in 0..2 {
                assert_eq!(release_unowned_submission_after_drive_failure(&mut shard, run), Ok(()));
                assert_eq!(shard.global_cost_usage(), 7);
                assert_eq!(shard.charged_quotas, quota);
                assert_eq!(shard.usage_snapshot(), usage);
                assert_eq!(shard.journal_seq_get(run), Some(EventSeq::new(100)));
                assert_eq!(shard.runtime_state_get(run), runtime_state);
                assert_eq!(shard.checked_out_run_contains(run), checked_out);
            }
        }
        Ok(())
    }

    #[test]
    fn failed_submit_cleanup_releases_unowned_and_terminal_only_accounting() -> TestResult {
        for terminal in [false, true] {
            let mut shard = test_shard()?;
            let run = RunId::new(72_102);
            let empty_usage = shard.usage_snapshot();
            if terminal {
                shard.checked_out_run_insert(run)?;
                shard.terminal_runs_insert(run)?;
            }
            shard.charge_admission_quota(run, Some(vb_core::ids::TenantId::new(93)), 7)?;
            shard.seed_journal_sequence(run, EventSeq::new(100))?;
            for _ in 0..2 {
                assert_eq!(release_unowned_submission_after_drive_failure(&mut shard, run), Ok(()));
                assert_eq!(shard.global_cost_usage(), 0);
                assert!(!shard.charged_quotas.contains_key(&run));
                assert_eq!(shard.usage_snapshot(), empty_usage);
                assert_eq!(shard.journal_seq_get(run), None);
                assert_eq!(shard.terminal_runs_contains(run), terminal);
            }
        }
        Ok(())
    }
