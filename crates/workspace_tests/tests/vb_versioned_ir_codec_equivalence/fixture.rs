//! Fixture factory and wire oracle for versioned IR codec equivalence tests.

use blake3::Hash;
use postcard::to_allocvec;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, VersionedWorkflowParts,
    WorkflowIrV2, WorkflowParts,
};

const V2_PREFIX: [u8; 4] = [1, 255, 86, 66];

fn value_node() -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
        Some(SlotIdx::ZERO),
        Some(StepIdx::new(1)),
    )
}

fn finish_node() -> CompiledNode {
    vb_core::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    )
}

fn constants(count: u32) -> Box<[ConstValue]> {
    if count == 0 {
        Box::from([ConstValue::I64(7)])
    } else {
        Box::from([ConstValue::Symbol(SymbolId::new(0))])
    }
}

fn make_parts(count: u32) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("codec-fixture"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: Box::from([value_node(), finish_node()]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: constants(count),
        slot_count: 1,
        symbols_count: count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([Box::from("set"), Box::from("finish")]),
        input_slots: Box::new([]),
    }
}

pub(super) fn legacy(count: u32) -> Result<VersionedWorkflowParts, Box<dyn std::error::Error>> {
    let parts = make_parts(count);
    Ok(VersionedWorkflowParts::V1(parts))
}

pub(super) fn bound(values: &[&str]) -> Result<VersionedWorkflowParts, Box<dyn std::error::Error>> {
    let count = u32::try_from(values.len()).map_err(|_| "symbol count too large for u32")?;
    let parts = make_parts(count);
    let payloads: Box<[Box<str>]> = values.iter().map(|v| Box::<str>::from(*v)).collect();
    let ir = WorkflowIrV2 {
        wire_version: 2,
        parts,
        symbol_payloads: payloads,
    };
    Ok(VersionedWorkflowParts::V2(ir))
}

pub(super) fn independent_wire(
    versioned: &VersionedWorkflowParts,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    match versioned {
        VersionedWorkflowParts::V1(parts) => {
            let bytes = to_allocvec(parts).map_err(|e| format!("postcard V1 encode: {e}"))?;
            Ok(bytes)
        }
        VersionedWorkflowParts::V2(ir) => {
            let body = to_allocvec(ir).map_err(|e| format!("postcard V2 body: {e}"))?;
            let mut wire = V2_PREFIX.to_vec();
            wire.extend_from_slice(&body);
            Ok(wire)
        }
    }
}

fn zeroed_versioned(versioned: &VersionedWorkflowParts) -> VersionedWorkflowParts {
    match versioned {
        VersionedWorkflowParts::V1(parts) => {
            let mut zeroed = parts.clone();
            zeroed.digest = WorkflowDigest::from_bytes([0; 32]);
            VersionedWorkflowParts::V1(zeroed)
        }
        VersionedWorkflowParts::V2(ir) => {
            let mut zeroed_parts = ir.parts.clone();
            zeroed_parts.digest = WorkflowDigest::from_bytes([0; 32]);
            VersionedWorkflowParts::V2(WorkflowIrV2 {
                wire_version: ir.wire_version,
                parts: zeroed_parts,
                symbol_payloads: ir.symbol_payloads.clone(),
            })
        }
    }
}

pub(super) fn stamp(
    versioned: VersionedWorkflowParts,
) -> Result<VersionedWorkflowParts, Box<dyn std::error::Error>> {
    let zeroed = zeroed_versioned(&versioned);
    let preimage = independent_wire(&zeroed)?;
    let hash: Hash = blake3::hash(&preimage);
    let digest = WorkflowDigest::from_bytes(hash.into());

    match versioned {
        VersionedWorkflowParts::V1(mut parts) => {
            parts.digest = digest;
            Ok(VersionedWorkflowParts::V1(parts))
        }
        VersionedWorkflowParts::V2(ir) => {
            let mut stamped_ir = ir;
            stamped_ir.parts.digest = digest;
            Ok(VersionedWorkflowParts::V2(stamped_ir))
        }
    }
}

pub(super) fn checked_legacy(
    parts: WorkflowParts,
) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    Ok(CompiledWorkflow::try_from_parts(parts)
        .map_err(|e| format!("V1 workflow reconstruction: {e}"))?)
}

pub(super) fn checked_bound(
    versioned: &VersionedWorkflowParts,
) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    let ir = match versioned {
        VersionedWorkflowParts::V2(ir) => ir,
        _ => return Err("checked_bound requires V2".into()),
    };
    Ok(
        CompiledWorkflow::try_from_parts_with_symbols(ir.parts.clone(), ir.symbol_payloads.clone())
            .map_err(|e| format!("V2 workflow reconstruction: {e}"))?,
    )
}
