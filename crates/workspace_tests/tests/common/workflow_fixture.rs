#![allow(dead_code)] // Per-target subsets: each test root imports only the builders it calls.
//! Canonical workflow builders hoisted from per-file copies.
//!
//! The `node` / `workflow_from_parts` / `finished_workflow` /
//! `action_then_finish_workflow` helpers below were byte-identical (modulo
//! name/digest/action parameters, plus vt2f's Finish output slot) across
//! the migrated test roots. Builders with content-hash digests keep their
//! local copies: unifying them would change workflow bytes.

use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract};
use vb_core::workflow::{WorkflowParts, node as workflow_node};

pub(crate) fn node(
    id: u16,
    output: Option<u16>,
    next: Option<u16>,
    kind: CompiledNodeKind,
) -> CompiledNode {
    workflow_node(
        StepIdx::new(id),
        kind,
        output.map(SlotIdx::new),
        next.map(StepIdx::new),
    )
}

pub(crate) fn workflow_from_parts(
    name: &str,
    digest_byte: u8,
    nodes: Box<[CompiledNode]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
) -> Result<CompiledWorkflow, String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::from(name),
        digest: WorkflowDigest::from_bytes([digest_byte; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants,
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts)
        .map_err(|err| format!("workflow fixture {name} invalid: {err:?}"))
}

pub(crate) fn finished_workflow(name: &str, digest_byte: u8) -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        name,
        digest_byte,
        Box::from([
            node(
                0,
                Some(0),
                Some(1),
                CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
            ),
            node(
                1,
                None,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            ),
        ]),
        Box::from([ConstValue::Bool(true)]),
        1,
    )
}

pub(crate) fn action_then_finish_workflow(
    name: &str,
    digest_byte: u8,
    action: ActionId,
    finish_output: Option<u16>,
) -> Result<CompiledWorkflow, String> {
    workflow_from_parts(
        name,
        digest_byte,
        Box::from([
            node(
                0,
                Some(1),
                Some(1),
                CompiledNodeKind::Do {
                    action,
                    input: SlotIdx::ZERO,
                },
            ),
            node(
                1,
                finish_output,
                None,
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            ),
        ]),
        Box::from([]),
        2,
    )
}
