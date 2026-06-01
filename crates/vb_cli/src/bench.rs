//! Benchmark and diagnostic commands for velvet-ballistics.
#![forbid(unsafe_code)]

use std::path::Path;
use std::process::ExitCode;
use vb_core::{RunId, WorkflowDigest};

pub(crate) fn cmd_bench_run(workflow: &Path) -> ExitCode {
    let bytes = match read_file(workflow) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let compile_start = std::time::Instant::now();
    let compiled = match vb_compile::compile_workflow(&bytes) {
        Ok(c) => c,
        Err(errors) => {
            for err in &errors.0 {
                crate::errln!("compile error: {err}");
            }
            return ExitCode::FAILURE;
        }
    };
    let compile_elapsed = compile_start.elapsed();

    let run_start = std::time::Instant::now();
    let run_id = RunId::new(1);
    let Some(shard_count) = std::num::NonZeroUsize::new(1) else {
        crate::errln!("runtime configuration error: shard count must be non-zero");
        return ExitCode::FAILURE;
    };
    let config = vb_runtime::shard::ShardConfig::default();
    let mut runtime = vb_runtime::runtime::Runtime::new(shard_count, config);
    if let Err(e) = runtime.submit_compiled(run_id, compiled) {
        crate::errln!("runtime submit error: {e}");
        return ExitCode::FAILURE;
    }
    if let Err(e) = runtime.tick_all() {
        crate::errln!("runtime tick error: {e}");
        return ExitCode::FAILURE;
    }
    let run_elapsed = run_start.elapsed();
    let counters = runtime.counters_snapshot();

    crate::outln!("compile: {}us", compile_elapsed.as_micros());
    crate::outln!("execute: {}us", run_elapsed.as_micros());
    crate::outln!(
        "total:   {}us",
        compile_elapsed.as_micros().saturating_add(run_elapsed.as_micros())
    );
    crate::outln!(
        "runtime: submitted={} completed={} failed={} steps={}",
        counters.runs_submitted,
        counters.runs_completed,
        counters.runs_failed,
        counters.steps_executed
    );

    if counters.runs_failed != 0 {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

pub(crate) fn cmd_doctor(db: &Path) -> ExitCode {
    let journal = match vb_storage::FjallJournal::open(db, None) {
        Ok(j) => j,
        Err(e) => {
            crate::errln!("FAIL: cannot open journal at {}: {e}", db.display());
            return ExitCode::FAILURE;
        }
    };
    crate::outln!("OK: journal opened at {}", db.display());

    match journal.persist_strict() {
        Ok(()) => crate::outln!("OK: strict persist succeeded"),
        Err(e) => {
            crate::errln!("FAIL: strict persist failed: {e}");
            return ExitCode::FAILURE;
        }
    }

    let test_run = RunId::new(unique_doctor_run_id());
    let test_event = vb_storage::JournalEvent::RunAccepted {
        run: test_run,
        seq: vb_storage::EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0xAB; 32]),
    };

    if let Err(e) = journal.append_journaled(&test_event) {
        crate::errln!("FAIL: cannot append test event: {e}");
        return ExitCode::FAILURE;
    }
    crate::outln!("OK: journal append succeeded");

    match journal.events_for_run(test_run) {
        Ok(events) => {
            if events.is_empty() {
                crate::errln!("FAIL: test event not found after append");
                return ExitCode::FAILURE;
            }
            crate::outln!("OK: journal read-back returned {} event(s)", events.len());
        }
        Err(e) => {
            crate::errln!("FAIL: cannot read test run events: {e}");
            return ExitCode::FAILURE;
        }
    }

    crate::outln!("doctor: all checks passed");
    ExitCode::SUCCESS
}

fn read_file(path: &std::path::Path) -> Result<Vec<u8>, ExitCode> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            crate::errln!("error reading {}: {e}", path.display());
            Err(ExitCode::FAILURE)
        }
    }
}

fn unique_doctor_run_id() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(now.as_nanos()).unwrap_or(now.as_secs())
}
