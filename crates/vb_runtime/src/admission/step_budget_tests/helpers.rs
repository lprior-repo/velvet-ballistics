//! Shared test helpers for the step-budget gate tests.
//!
//! These helpers were extracted from the original `step_budget_tests.rs`
//! (which exceeded the 300-line source cap) so the test files can stay
//! under the cap. All helpers are public to the parent test module.

#![allow(dead_code)]

use std::num::NonZeroUsize;

use vb_core::ids::{StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

use crate::Runtime;
use crate::shard::ShardConfig;

/// Builds a `CompiledWorkflow` with `max_steps` declared in the resource
/// contract. The compiled node graph is a single `Nop` node so this helper
/// isolates declared-contract admission behavior.
pub(crate) fn workflow_with_max_steps(max_steps: u16) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("max_steps_{max_steps}").into(),
        digest: WorkflowDigest::from_bytes([0xA0; 32]),
        nodes: Box::from([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract {
            max_steps,
            max_slots: 1,
            ..vb_core::workflow::ResourceContract::DEFAULT
        },
        step_names: linear_step_names(1),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

pub(crate) fn linear_workflow_with_declared_steps(
    node_count: u16,
    declared_max_steps: u16,
) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: format!("actual_nodes_{node_count}_declared_{declared_max_steps}").into(),
        digest: WorkflowDigest::from_bytes([0xB0; 32]),
        nodes: linear_nodes(node_count),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract {
            max_steps: declared_max_steps,
            max_slots: node_count,
            ..vb_core::workflow::ResourceContract::DEFAULT
        },
        step_names: linear_step_names(node_count),
    };
    CompiledWorkflow::from_parts_unchecked(parts)
}

pub(crate) fn linear_nodes(node_count: u16) -> Box<[CompiledNode]> {
    let mut nodes: Vec<CompiledNode> = Vec::with_capacity(usize::from(node_count));
    for index in 0..node_count {
        let kind = if next_linear_step(index, node_count).is_none() {
            CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            }
        } else {
            CompiledNodeKind::Nop
        };
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: next_linear_step(index, node_count),
            on_error: None,
            error_slot: None,
            kind,
        });
    }
    nodes.into_boxed_slice()
}

pub(crate) fn next_linear_step(index: u16, node_count: u16) -> Option<StepIdx> {
    match index.checked_add(1) {
        Some(next) if next < node_count => Some(StepIdx::new(next)),
        _ => None,
    }
}

pub(crate) fn linear_step_names(node_count: u16) -> Box<[Box<str>]> {
    let mut names = Vec::with_capacity(usize::from(node_count));
    let mut index = 0u16;
    while index < node_count {
        names.push(format!("s{index}").into_boxed_str());
        index = index.saturating_add(1);
    }
    names.into_boxed_slice()
}

pub(crate) fn master_step_limit_u16() -> u16 {
    match u16::try_from(vb_core::limits::MAX_STEPS_PER_WORKFLOW) {
        Ok(value) => value,
        Err(_) => u16::MAX,
    }
}

pub(crate) fn first_step_count_over_master_limit() -> u16 {
    match vb_core::limits::MAX_STEPS_PER_WORKFLOW.checked_add(1) {
        Some(value) => match u16::try_from(value) {
            Ok(converted) => converted,
            Err(_) => u16::MAX,
        },
        None => u16::MAX,
    }
}

pub(crate) fn total_command_queue_depth(runtime: &Runtime) -> u32 {
    runtime
        .collect_metrics()
        .shards
        .iter()
        .fold(0u32, |total, shard| {
            total.saturating_add(shard.command_queue_depth)
        })
}

/// Builds a runtime configured for strict admission with an always-present
/// artifact store so the step-budget gate is the only constraint that fires.
pub(crate) fn runtime_with_policy(policy: RuntimePolicy) -> Runtime {
    let config = ShardConfig {
        policy,
        ..ShardConfig::default()
    };
    Runtime::new_with_artifact_store(
        nonzero_one(),
        config,
        crate::admission::AlwaysPresentArtifactStore::shared(),
    )
}

fn nonzero_one() -> NonZeroUsize {
    match NonZeroUsize::new(1) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    }
}
