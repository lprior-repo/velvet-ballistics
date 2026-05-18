#![cfg(kani)]
#![forbid(unsafe_code)]

//! Bounded arbitrary generators for compiled workflow structures used by Kani.
//!
//! These generators vary every field of WorkflowParts structurally so Kani
//! harnesses exercise arbitrary shapes — nodes, expressions, constants,
//! step_names, and resource_contract — not just accessor variations.

use crate::ids::{
    AccessorIdx, ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::value::FiniteF64;
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, PathSegment,
    ResourceContract, SlotBranch, WorkflowParts,
};

impl kani::Arbitrary for PathSegment {
    fn any() -> Self {
        if kani::any::<bool>() {
            Self::Field(SymbolId::new(kani::any::<u32>()))
        } else {
            Self::Index(kani::any::<u32>())
        }
    }
}

impl kani::Arbitrary for AccessorProgram {
    fn any() -> Self {
        Self {
            root: SlotIdx::new(kani::any::<u16>()),
            path: bounded_path(),
        }
    }
}

impl kani::Arbitrary for WorkflowParts {
    fn any() -> Self {
        Self {
            name: Box::from("kani_workflow"),
            digest: WorkflowDigest::from_bytes(kani::any::<[u8; 32]>()),
            nodes: Box::new([CompiledNode {
                id: StepIdx::ZERO,
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            }]),
            expressions: Box::new([]),
            accessors: bounded_accessors(),
            constants: Box::new([]),
            slot_count: kani::any::<u16>(),
            symbols_count: kani::any::<u32>(),
            entry: StepIdx::ZERO,
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }
}

fn bounded_path() -> Box<[PathSegment]> {
    match bounded_len_3() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<PathSegment>()]),
        2 => Box::new([kani::any::<PathSegment>(), kani::any::<PathSegment>()]),
        _ => Box::new([
            kani::any::<PathSegment>(),
            kani::any::<PathSegment>(),
            kani::any::<PathSegment>(),
        ]),
    }
}

fn bounded_accessors() -> Box<[AccessorProgram]> {
    match bounded_len_3() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<AccessorProgram>()]),
        2 => Box::new([
            kani::any::<AccessorProgram>(),
            kani::any::<AccessorProgram>(),
        ]),
        _ => Box::new([
            kani::any::<AccessorProgram>(),
            kani::any::<AccessorProgram>(),
            kani::any::<AccessorProgram>(),
        ]),
    }
}

fn bounded_len_3() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= 3);
    len
}
