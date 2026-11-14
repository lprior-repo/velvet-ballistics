#![forbid(unsafe_code)]
//! Shared fixtures and helpers for the Together shipping scenarios.

use core::num::NonZeroUsize;
use std::sync::Arc;

use vb_compile::compile_workflow;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, SlotValue, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{FjallJournal, JournalEvent};

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(crate) const RUN: RunId = RunId::new(8_101);

/// Three declared branches with distinguishable results, no external inputs.
pub(crate) const FANOUT: &str = r#"version: velvet-ballistics/v1
name: together_shipping_fanout
when: {manual: {}}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: fanout
    together:
      branches:
        - label: first
          steps:
            - id: first_value
              set: {output: first_result, value: '7'}
        - label: second
          steps:
            - id: second_value
              set: {output: second_result, value: '11'}
        - label: third
          steps:
            - id: third_value
              set: {output: third_result, value: '13'}
  - id: done
    finish: {result: '$step.fanout'}
"#;

/// First branch reads a secret input; the second is a constant branch.
pub(crate) const TAINTED_FANOUT: &str = r#"version: velvet-ballistics/v1
name: together_shipping_taint
when: {manual: {}}
inputs:
  hidden: {is: number, secret: true}
steps:
  - id: fanout
    together:
      branches:
        - label: secret_branch
          steps:
            - id: hidden_value
              set: {output: hidden_result, value: '$input.hidden'}
        - label: clean_branch
          steps:
            - id: clean_value
              set: {output: clean_result, value: '11'}
  - id: done
    finish: {result: '$step.fanout'}
"#;

/// Second branch fails; the handler on the Together step must take over.
pub(crate) const FAILING_FANOUT: &str = r#"version: velvet-ballistics/v1
name: together_shipping_failure
when: {manual: {}}
inputs:
  zero: {is: number, secret: true}
steps:
  - id: seed
    set: {output: saved, value: '42'}
  - id: fanout
    on_error: {handler: recovered}
    together:
      branches:
        - label: ready
          steps:
            - id: ready_value
              set: {output: ready_result, value: '7'}
        - label: failing
          steps:
            - id: fault
              set: {output: fault_result, value: '1 / $input.zero'}
  - id: unreachable
    set: {output: skipped, value: '99'}
  - id: recovered
    set: {output: fallback, value: '$saved'}
  - id: done
    finish: {result: '$step.recovered'}
"#;

pub(crate) struct Live {
    pub(crate) journal: Arc<FjallJournal>,
    pub(crate) directory: tempfile::TempDir,
    pub(crate) plan: CompiledWorkflow,
    pub(crate) state: vb_runtime::shard::types::RunState,
}

pub(crate) fn open_journal(directory: &tempfile::TempDir) -> TestResult<Arc<FjallJournal>> {
    Ok(Arc::new(FjallJournal::open(directory.path(), None)?))
}

pub(crate) fn live(source: &str, inputs: Box<[(SlotIdx, SlotValue)]>) -> TestResult<Live> {
    let plan = compile_workflow(source.as_bytes()).map_err(|error| format!("{error:?}"))?;
    let directory = tempfile::tempdir()?;
    let journal = open_journal(&directory)?;
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal.clone()),
    );
    if inputs.is_empty() {
        runtime.submit_compiled(RUN, plan.clone())?;
    } else {
        runtime.submit_compiled_with_inputs(RUN, plan.clone(), inputs)?;
    }
    let mut state = None;
    let mut completed = false;
    for _ in 0..256 {
        runtime.tick_all()?;
        if let Ok(current) = runtime.inspect_run_state(RUN) {
            state = Some(current.clone());
        }
        let counters = runtime.counters_snapshot();
        assert_eq!(
            counters.runs_failed,
            0,
            "live run failed: {:?}",
            journal.events_for_run_full(RUN)?
        );
        if counters.runs_completed == 1 {
            completed = true;
            break;
        }
    }
    assert!(completed, "live Together run did not terminate");
    Ok(Live {
        journal,
        directory,
        plan,
        state: state.ok_or("live run vanished before any inspectable state")?,
    })
}

pub(crate) fn events(live: &Live) -> TestResult<Vec<JournalEvent>> {
    Ok(live.journal.events_for_run_full(RUN)?)
}

/// Persists a rewritten event prefix in a fresh store for rejection probes.
pub(crate) fn journal_with(
    events: &[JournalEvent],
) -> TestResult<(tempfile::TempDir, Arc<FjallJournal>)> {
    let directory = tempfile::tempdir()?;
    let journal = open_journal(&directory)?;
    for event in events {
        journal.append_strict(event)?;
    }
    Ok((directory, journal))
}

pub(crate) fn succeeded(events: &[JournalEvent]) -> Vec<vb_core::StepIdx> {
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::StepSucceeded { step, .. } => Some(*step),
            _ => None,
        })
        .collect()
}

pub(crate) fn failed(events: &[JournalEvent]) -> Vec<vb_core::StepIdx> {
    events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::StepFailed { step, .. } => Some(*step),
            _ => None,
        })
        .collect()
}

pub(crate) fn together_start(
    plan: &CompiledWorkflow,
) -> TestResult<(vb_core::StepIdx, Vec<vb_core::StepIdx>, vb_core::StepIdx)> {
    plan.to_parts()
        .nodes
        .iter()
        .find_map(|node| match &node.kind {
            vb_core::CompiledNodeKind::TogetherStart { branches, join } => {
                Some((node.id, branches.to_vec(), *join))
            }
            _ => None,
        })
        .ok_or_else(|| "TogetherStart node missing".into())
}

pub(crate) fn finish_result(plan: &CompiledWorkflow) -> TestResult<SlotIdx> {
    plan.to_parts()
        .nodes
        .iter()
        .find_map(|node| match node.kind {
            vb_core::CompiledNodeKind::Finish { result } => Some(result),
            _ => None,
        })
        .ok_or_else(|| "Finish result missing".into())
}

pub(crate) fn list_value(
    store: &vb_core::ValueStore,
    value: SlotValue,
) -> TestResult<Vec<SlotValue>> {
    let SlotValue::List(id) = value else {
        return Err("expected a list result".into());
    };
    Ok(store.list(id)?.to_vec())
}

pub(crate) fn list_taints(store: &vb_core::ValueStore, value: SlotValue) -> TestResult<Vec<Taint>> {
    let SlotValue::List(id) = value else {
        return Err("expected a list result".into());
    };
    (0..store.list(id)?.len())
        .map(|index| {
            let index = u32::try_from(index)?;
            Ok(store.list_item_with_taint(id, index)?.1)
        })
        .collect()
}

pub(crate) fn tainted_plan() -> TestResult<CompiledWorkflow> {
    compile_workflow(TAINTED_FANOUT.as_bytes()).map_err(|error| format!("{error:?}").into())
}

pub(crate) fn hidden_slot(plan: &CompiledWorkflow) -> TestResult<SlotIdx> {
    plan.input_slots()
        .first()
        .map(|input| input.slot)
        .ok_or_else(|| "secret input slot missing".into())
}
