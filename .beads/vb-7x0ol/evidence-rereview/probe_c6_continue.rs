// Temporary independent re-review probe (vb-7x0ol). NOT a production test.
// Extends the C6 retained-input recovery scenario to actually CONTINUE the
// recovered run and observe the exact finish result, which vb_md3w_retained_input_recovery.rs
// only checks as "Resumable" (never drives to completion).

use std::{fs, num::NonZeroUsize, path::Path, process::Command, sync::Arc};
use vb_core::RuntimePolicy;
use vb_core::{RunId, SlotIdx, SlotValue};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallConfig, FjallJournal, JournalEvent};

const INPUT_FINISH: &str = "version: velvet-ballistics/v1\nname: retained_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: done\n    finish:\n      result: $input.count\n";

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("invalid shard count {value}"))
}

fn journaled_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: RuntimePolicy::Journaled,
    }
}

fn submit(yaml: &Path, input: &Path, db: &Path) -> Result<RunId, Box<dyn std::error::Error>> {
    let result = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("submit")
        .arg(yaml)
        .arg("--input-bin")
        .arg(input)
        .arg("--db")
        .arg(db)
        .args(["--durability", "journaled", "--emit", "yaml"])
        .output()?;
    assert_eq!(
        result.status.code(),
        Some(0),
        "submit failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: serde_json::Value =
        serde_saphyr::from_str(&String::from_utf8(result.stdout)?)?;
    assert_eq!(
        receipt.get("status").and_then(serde_json::Value::as_str),
        Some("submitted")
    );
    Ok(RunId::new(
        receipt
            .get("run_id")
            .and_then(serde_json::Value::as_u64)
            .ok_or("missing run")?,
    ))
}

/// C6 continue: submit -> delete files -> cold reopen -> recover_and_resume ->
/// drive to completion -> the durable RunFinished result slot must be the
/// retained input slot (0), proving the recovered run continues with the exact
/// retained input value (not a default, not a journal-overlay erasure).
#[test]
fn recovered_run_continues_to_exact_finish_result_slot() -> TestResult {
    for value in [i64::MIN, -7, 0, 41, i64::MAX] {
        let directory = tempfile::tempdir()?;
        let yaml = directory.path().join("workflow.yaml");
        let input = directory.path().join("input.bin");
        let db = directory.path().join("journal");

        fs::write(&yaml, INPUT_FINISH)?;
        fs::write(&input, postcard::to_allocvec(&vec![SlotValue::I64(value)])?)?;

        let run = submit(&yaml, &input, &db)?;
        fs::remove_file(&yaml)?;
        fs::remove_file(&input)?;

        let fjall_journal = Arc::new(FjallJournal::open(&db, Some(FjallConfig::default()))?);
        let retained = fjall_journal
            .run_inputs(run)?
            .ok_or("missing retained inputs")?;
        let input_slot = retained
            .inputs()
            .iter()
            .find(|(slot, _)| *slot == SlotIdx::ZERO)
            .map(|(_, v)| *v)
            .ok_or("missing input slot 0")?;
        assert_eq!(input_slot, SlotValue::I64(value));

        let storage_journal = Arc::new(StorageRuntimeJournal::journaled(fjall_journal.clone()));
        let mut runtime = Runtime::new(shard_count(1)?, journaled_config(), storage_journal);
        runtime.recover_and_resume(run)?;
        runtime.tick_all()?;

        let events = fjall_journal.events_for_run_full(run)?;
        let finished = events
            .iter()
            .find_map(|e| match e {
                JournalEvent::RunFinished { result, .. } => Some(*result),
                _ => None,
            })
            .ok_or("expected RunFinished after recover_and_resume + tick")?;
        assert_eq!(
            finished,
            SlotIdx::ZERO,
            "finish result slot must reference the retained input slot"
        );
    }
    Ok(())
}
