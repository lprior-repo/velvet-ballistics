#![forbid(unsafe_code)]

use std::{num::NonZeroUsize, sync::Arc};
use vb_core::{RunId, RuntimePolicy};
use vb_ipc::{
    IpcCommand, IpcFrameHeader, IpcPayload,
    server::{IpcResponse, dispatch::dispatch_command_with_resolver},
};
use vb_runtime::{
    journal::StorageRuntimeJournal,
    runtime::Runtime,
    shard::{InspectResponse, ShardConfig},
    trace::TraceEvent,
};
use vb_storage::FjallJournal;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const TARGET: RunId = RunId::new(7001);
const OTHER: RunId = RunId::new(7002);
const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: scoped_trace\nwhen: {manual: {}}\nsteps:\n  - id: seed\n    set: {output: alias, value: '42'}\n  - id: clock\n    set: {output: duration, value: '3600'}\n  - id: pause\n    wait: {timeout: '$steps.clock'}\n  - id: done\n    finish: {result: '$seed'}\n";

struct Fixture {
    _directory: tempfile::TempDir,
    runtime: Runtime,
    target: Vec<TraceEvent>,
    other: Vec<TraceEvent>,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        let workflow = vb_compile::compile_workflow(SOURCE)?;
        vb_storage::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
        let mut runtime = Runtime::new(
            NonZeroUsize::new(2).ok_or("two shards required")?,
            ShardConfig {
                command_queue_capacity: 8,
                trace_capacity: 64,
                step_budget_per_tick: 1,
                max_active_runs: 4,
                max_open_gates: 4,
                policy: RuntimePolicy::Strict,
            },
            StorageRuntimeJournal::shared_strict(journal),
        )?;
        runtime.submit_direct(TARGET, workflow.clone())?;
        runtime.submit_direct(OTHER, workflow)?;
        assert_eq!(runtime.tick_all(), Ok(true));
        assert!(matches!(
            runtime.snapshot_run(TARGET, 0)?,
            InspectResponse::Found(_)
        ));
        assert!(matches!(
            runtime.snapshot_run(OTHER, 0)?,
            InspectResponse::Found(_)
        ));
        assert_eq!(runtime.counters_snapshot().runs_submitted, 2);
        assert_eq!(runtime.counters_snapshot().runs_failed, 0);
        let target = runtime.list_events(TARGET)?;
        let other = runtime.list_events(OTHER)?;
        assert!(target.len() > 2);
        assert!(other.len() > 2);
        assert!(target.iter().all(|event| event.run_id() == TARGET));
        assert!(other.iter().all(|event| event.run_id() == OTHER));
        assert_ne!(target.first(), target.get(1));
        Ok(Self {
            _directory: directory,
            runtime,
            target,
            other,
        })
    }

    fn drain(&mut self, run_id: RunId, max_records: u32) -> TestResult<IpcResponse> {
        let payload = postcard::to_allocvec(&IpcPayload::DrainTrace {
            run_id,
            max_records,
        })?;
        Ok(self.dispatch(&payload))
    }

    fn dispatch(&mut self, payload: &[u8]) -> IpcResponse {
        let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 11, 0);
        dispatch_command_with_resolver(&header, payload, &mut self.runtime, None)
    }

    fn remaining(&self, target_skip: usize, other_skip: usize) -> Vec<TraceEvent> {
        // OTHER belongs to shard 0; TARGET belongs to shard 1.
        self.other
            .iter()
            .skip(other_skip)
            .chain(self.target.iter().skip(target_skip))
            .cloned()
            .collect()
    }

    fn assert_history(&self) -> TestResult {
        assert_eq!(self.runtime.list_events(TARGET)?, self.target);
        assert_eq!(self.runtime.list_events(OTHER)?, self.other);
        Ok(())
    }
}

#[test]
fn ipc_zero_preserves_all_pending_records_and_history() -> TestResult {
    let mut fixture = Fixture::new()?;
    assert_eq!(
        fixture.drain(TARGET, 0)?,
        IpcResponse::TraceCount { count: 0 }
    );
    fixture.assert_history()?;
    let expected = fixture.remaining(0, 0);
    assert_eq!(fixture.runtime.drain_trace(), expected);
    assert_eq!(fixture.runtime.drain_trace(), Vec::new());
    Ok(())
}

#[test]
fn ipc_one_preserves_other_run_and_exact_target_tail() -> TestResult {
    let mut fixture = Fixture::new()?;
    assert_eq!(
        fixture.drain(TARGET, 1)?,
        IpcResponse::TraceCount { count: 1 }
    );
    fixture.assert_history()?;
    let expected = fixture.remaining(1, 0);
    assert_eq!(fixture.runtime.drain_trace(), expected);
    Ok(())
}

#[test]
fn ipc_repeated_limits_exhaust_only_target_and_keep_other_run() -> TestResult {
    let mut fixture = Fixture::new()?;
    assert_eq!(
        fixture.drain(TARGET, 1)?,
        IpcResponse::TraceCount { count: 1 }
    );
    assert_eq!(
        fixture.drain(TARGET, 1)?,
        IpcResponse::TraceCount { count: 1 }
    );
    let count = u32::try_from(
        fixture
            .target
            .len()
            .checked_sub(2)
            .ok_or("target too short")?,
    )?;
    assert_eq!(
        fixture.drain(TARGET, u32::MAX)?,
        IpcResponse::TraceCount { count }
    );
    assert_eq!(
        fixture.drain(TARGET, u32::MAX)?,
        IpcResponse::TraceCount { count: 0 }
    );
    assert_eq!(
        fixture.drain(OTHER, 1)?,
        IpcResponse::TraceCount { count: 1 }
    );
    fixture.assert_history()?;
    let expected = fixture.remaining(fixture.target.len(), 1);
    assert_eq!(fixture.runtime.drain_trace(), expected);
    Ok(())
}

#[test]
fn ipc_bad_request_and_unknown_run_preserve_pending_records() -> TestResult {
    let mut fixture = Fixture::new()?;
    assert_eq!(fixture.dispatch(b"invalid"), IpcResponse::BadRequest);
    assert_eq!(
        fixture.drain(RunId::new(9999), 1)?,
        IpcResponse::RuntimeError {
            message: String::from("run not found")
        }
    );
    fixture.assert_history()?;
    let expected = fixture.remaining(0, 0);
    assert_eq!(fixture.runtime.drain_trace(), expected);
    Ok(())
}

#[test]
fn runtime_scoped_drain_returns_exact_successive_records() -> TestResult {
    let mut fixture = Fixture::new()?;
    assert_eq!(fixture.runtime.drain_trace_for_run(TARGET, 0), Vec::new());
    assert_eq!(
        fixture
            .runtime
            .drain_trace_for_run(RunId::new(9999), usize::MAX),
        Vec::new()
    );
    assert_eq!(
        fixture.runtime.drain_trace_for_run(TARGET, 1),
        fixture.target.iter().take(1).cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        fixture.runtime.drain_trace_for_run(TARGET, 1),
        fixture
            .target
            .iter()
            .skip(1)
            .take(1)
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        fixture.runtime.drain_trace_for_run(TARGET, usize::MAX),
        fixture.target.iter().skip(2).cloned().collect::<Vec<_>>()
    );
    assert_eq!(
        fixture.runtime.drain_trace_for_run(TARGET, usize::MAX),
        Vec::new()
    );
    fixture.assert_history()?;
    assert_eq!(fixture.runtime.drain_trace(), fixture.other);
    Ok(())
}
