//! Owned shipping harness for nested outer/inner control cells.
//!
//! Bead: vb-eirmk.4.7.4. Builds real YAML from the frozen shared fixture,
//! admits it through the owned submission API on a strict Fjall journal,
//! drives the live runtime to a nonterminal cut inside the nested body,
//! drops everything, and proves recovery/resume/replay parity.

#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::frame::RunFrameCheckpointV1;
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::value_store::ValueStoreCheckpointV1;
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, ConstValue, RunId, RuntimePolicy, SlotIdx, SlotValue,
    StepIdx, StepState, Taint, ValueStore, WorkflowDigest,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::{
    FjallJournal, JournalEvent, OwnedRunInputsRecordV1, RunHeaderRecord, RunSubmission,
    WorkflowSourceRecord,
};

#[path = "support/owned.rs"]
mod owned;
use crate::source::{compound, indent};
pub(crate) use owned::{Cut, cut_at_leaf};

pub(crate) type TestResult<T = ()> = Result<T, String>;

pub(crate) const INPUT_VALUES: [SlotValue; 2] = [SlotValue::I64(3), SlotValue::I64(5)];
pub(crate) const INPUT_TAINTS: [Taint; 2] = [Taint::Secret, Taint::DerivedFromSecret];
pub(crate) const SENTINEL: i64 = 42;
pub(crate) const LEAF: i64 = 9;

/// One legal or prohibited matrix cell with its distinct run id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) outer: &'static str,
    pub(crate) inner: &'static str,
    pub(crate) run: RunId,
}

impl Cell {
    pub(crate) const fn new(outer: &'static str, inner: &'static str, run: u64) -> Self {
        Self {
            outer,
            inner,
            run: RunId::new(run),
        }
    }

    pub(crate) fn label(self) -> String {
        format!("{}/{}", self.outer, self.inner)
    }

    pub(crate) fn prohibited(self) -> bool {
        matches!((self.outer, self.inner), ("together", "together"))
    }
}

fn family_index(family: &str) -> u64 {
    match family {
        "repeat" => 0,
        "for_each" => 1,
        "collect" => 2,
        "reduce" => 3,
        "together" => 4,
        "choose" => 5,
        _ => 6,
    }
}

/// Builds a cell with a distinct, deterministic run id.
pub(crate) fn cell(outer: &'static str, inner: &'static str) -> Cell {
    let encoded = family_index(outer)
        .checked_mul(8)
        .and_then(|outer| outer.checked_add(family_index(inner)));
    let run = match encoded {
        Some(offset) => match offset.checked_add(7_400) {
            Some(run) => run,
            None => 7_400,
        },
        None => 7_400,
    };
    Cell::new(outer, inner, run)
}

/// Documented multiplicity of a compound family body (shared component oracle).
pub(crate) fn multiplicity(family: &str) -> usize {
    match family {
        "repeat" | "for_each" | "collect" | "reduce" => 2,
        _ => 1,
    }
}

const INNER_BODY: &str = "- id: leaf\n  set: {output: leaf, value: '9'}\n";
const TAIL: &str = "- id: tail\n  set: {output: tail, value: '$vars.sentinel'}\n";

/// Builds the real source for one cell from the frozen shared fixture helpers.
pub(crate) fn source_for(cell: Cell) -> TestResult<String> {
    let inner_body = compound(cell.inner, "inner", INNER_BODY)?;
    let outer_body = format!("{inner_body}{TAIL}");
    let outer = compound(cell.outer, "outer", &outer_body)?;
    Ok(format!(
        "version: velvet-ballistics/v1\nname: shipping_nested_control_{}_{}\nwhen: {{manual: {{}}}}\ninputs:\n  items: {{is: list, of: any, max: 4}}\nvars: {{sentinel: 42, marker: source_payload}}\nsteps:\n{}  - id: done\n    finish: {{result: '$vars.sentinel'}}\n",
        cell.outer,
        cell.inner,
        indent(&outer, 2)
    ))
}

pub(crate) fn workflow_for(source: &str) -> TestResult<CompiledWorkflow> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())
        .map_err(|errors| format!("compile: {errors:?}"))?;
    let parts = workflow.to_parts();
    let revalidated = CompiledWorkflow::try_from_parts(parts.clone())
        .map_err(|error| format!("full IR revalidation: {error:?}"))?;
    if revalidated.to_parts() != parts {
        return Err("full IR round trip changed parts".to_owned());
    }
    Ok(workflow)
}

pub(crate) fn step_named(workflow: &CompiledWorkflow, name: &str) -> TestResult<StepIdx> {
    (0..workflow.node_count())
        .map(StepIdx::new)
        .find(|step| workflow.step_name(*step) == Some(name))
        .ok_or_else(|| format!("missing step {name}"))
}

pub(crate) fn leaf_step(workflow: &CompiledWorkflow) -> TestResult<StepIdx> {
    (0..workflow.node_count())
        .map(StepIdx::new)
        .find(|step| {
            workflow.node(*step).is_some_and(|node| {
                matches!(node.kind, K::SetConst { value }
                    if workflow.constant(value) == Some(&ConstValue::I64(LEAF)))
            })
        })
        .ok_or_else(|| "missing authored leaf SetConst(9)".to_owned())
}
