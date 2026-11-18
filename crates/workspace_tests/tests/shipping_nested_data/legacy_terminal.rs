use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::{RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::FjallJournal;

use super::support::{TestResult, terminal};

fn runtime(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

fn direct_terminal_cold_recovery(source: &str, run: RunId) -> TestResult {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    vb_core::validate_compiled_workflow(&workflow.to_parts())?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    vb_storage::admission::submit_artifact(&journal, &workflow, RuntimePolicy::Strict)?;
    {
        let mut runtime = runtime(journal.clone());
        runtime.submit_compiled(run, workflow.clone())?;
        for _ in 0..1024 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_completed == 1 {
                break;
            }
            if runtime.counters_snapshot().runs_failed != 0 {
                return Err(format!(
                    "direct loop run failed before terminal completion: {:?}",
                    runtime.list_events(run)
                )
                .into());
            }
        }
        if runtime.counters_snapshot().runs_completed != 1 {
            return Err("direct loop run did not reach terminal completion".into());
        }
    }
    let baseline = journal.events_for_run_full(run)?;
    let result = terminal(&baseline)?;
    if result != (SlotValue::I64(42), Taint::Clean) {
        return Err(format!("direct loop terminal result diverged: {result:?}").into());
    }
    drop(journal);

    let reopened = Arc::new(FjallJournal::open(directory.path(), None)?);
    let runtime = runtime(reopened.clone());
    if reopened.events_for_run_full(run)? != baseline {
        return Err("terminal journal changed during cold reopen".into());
    }
    let product = runtime.recover_product(run)?;
    match product {
        vb_runtime::recovery::RuntimeRecoveryProduct::CannotResume(reason) => {
            if reason.reason() != "terminal_state" {
                return Err(format!("unexpected terminal reason {}", reason.reason()).into());
            }
        }
        other => return Err(format!("terminal direct run recovered as {other:?}").into()),
    }
    if reopened.events_for_run_full(run)? != baseline {
        return Err("terminal classification mutated the journal".into());
    }
    if runtime.recover_and_resume(run)
        != Err(vb_runtime::RuntimeError::RecoveryCannotResume {
            reason: String::from("terminal_state"),
        })
    {
        return Err("terminal direct run did not refuse resume exactly".into());
    }
    if reopened.events_for_run_full(run)? != baseline {
        return Err("terminal resume refusal mutated the journal".into());
    }
    Ok(())
}

#[test]
fn direct_terminal_for_each_classifies_before_input_replay() -> TestResult {
    direct_terminal_cold_recovery(
        "version: velvet-ballistics/v1\nname: terminal_direct_foreach\nwhen: {manual: {}}\nvars: {items: [1, 2, 3]}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: outer\n    for_each:\n      variable: entry\n      input: '$vars.items'\n      at_once: 1\n      steps:\n        - id: body\n          set: {output: seen, value: '$entry'}\n  - id: done\n    finish: {result: '$step.seed'}\n",
        RunId::new(7_121),
    )
}

#[test]
fn direct_terminal_collect_classifies_before_input_replay() -> TestResult {
    direct_terminal_cold_recovery(
        "version: velvet-ballistics/v1\nname: terminal_direct_collect\nwhen: {manual: {}}\nvars: {items: [1, 2, 3]}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: outer\n    collect:\n      variable: batch\n      source: '$vars.items'\n      pages: 4\n      items: 1\n      steps:\n        - id: body\n          set: {output: copied, value: '$batch'}\n  - id: done\n    finish: {result: '$step.seed'}\n",
        RunId::new(7_122),
    )
}

#[test]
fn direct_terminal_reduce_classifies_before_input_replay() -> TestResult {
    direct_terminal_cold_recovery(
        "version: velvet-ballistics/v1\nname: terminal_direct_reduce\nwhen: {manual: {}}\nvars: {items: [1, 2, 3]}\nsteps:\n  - id: seed\n    set: {output: seed, value: '42'}\n  - id: outer\n    reduce:\n      variable: accumulator\n      input: '$vars.items'\n      initial: '0'\n      steps:\n        - id: body\n          set: {output: running, value: '$accumulator + 1'}\n  - id: done\n    finish: {result: '$step.seed'}\n",
        RunId::new(7_123),
    )
}
