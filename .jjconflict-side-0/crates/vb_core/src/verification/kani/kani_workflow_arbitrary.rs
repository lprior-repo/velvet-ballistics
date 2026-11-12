#![cfg(kani)]
#![forbid(unsafe_code)]

//! Bounded arbitrary generators for compiled workflow structures used by Kani.
//!
//! These generators vary every field of WorkflowParts structurally so Kani
//! harnesses exercise arbitrary shapes — nodes, expressions, constants,
//! step_names, and resource_contract — not just accessor variations.

use crate::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use crate::capability::Capability;
use crate::frame::RunFrame;
use crate::ids::{
    AccessorIdx, ActionId, BlobId, ConstIdx, ExprIdx, ListId, ObjectId, SlotIdx, StepIdx, SymbolId,
    WorkflowDigest,
};
use crate::value::{ConstValue, FiniteF64, SlotValue, Taint};
use crate::workflow::{
    AccessorProgram, CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind,
    ExprBranch, ExprOp, ExprProgram, ExtraFields, InputSlotKind, Nullability, PathSegment,
    Presence, ResourceContract, Secretness, SlotBranch, WorkflowParts,
};

/// Ordered optional i64 bounds with `min <= max` whenever both are present.
fn ordered_optional_bounds() -> (Option<i64>, Option<i64>) {
    let (first, second): (i64, i64) = (kani::any(), kani::any());
    let (present_first, present_second): (bool, bool) = (kani::any(), kani::any());
    let (low, high) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    (present_first.then_some(low), present_second.then_some(high))
}

/// Ordered optional non-negative i64 bounds for length constraints.
fn ordered_optional_length_bounds() -> (Option<i64>, Option<i64>) {
    let (first, second): (u32, u32) = (kani::any(), kani::any());
    let (present_first, present_second): (bool, bool) = (kani::any(), kani::any());
    let (low, high) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    (
        present_first.then_some(i64::from(low)),
        present_second.then_some(i64::from(high)),
    )
}

impl kani::Arbitrary for FiniteF64 {
    fn any() -> Self {
        FiniteF64::_kani_any()
    }
}

impl kani::Arbitrary for ExprOp {
    fn any() -> Self {
        match kani::any::<u8>() {
            0 => Self::LoadSlot(SlotIdx::new(kani::any())),
            1 => Self::LoadConst(ConstIdx::new(kani::any())),
            2 => Self::LoadAccessor(AccessorIdx::new(kani::any())),
            3 => Self::Eq,
            4 => Self::NotEq,
            5 => Self::Gt,
            6 => Self::Gte,
            7 => Self::Lt,
            8 => Self::Lte,
            9 => Self::And,
            10 => Self::Or,
            11 => Self::Not,
            12 => Self::Add,
            13 => Self::Sub,
            14 => Self::Mul,
            15 => Self::Div,
            16 => Self::Contains,
            17 => Self::StartsWith,
            18 => Self::EndsWith,
            19 => Self::Has,
            20 => Self::Exists,
            21 => Self::Length,
            22 => Self::Empty,
            23 => Self::Append,
            24 => Self::AppendIf,
            25 => Self::Merge,
            26 => Self::Sum,
            27 => Self::Count,
            28 => Self::Unique,
            _ => Self::LoadSlot(SlotIdx::new(kani::any())),
        }
    }
}

impl kani::Arbitrary for ExprProgram {
    fn any() -> Self {
        let ops_len: u8 = kani::any();
        kani::assume(ops_len <= 16);
        Self {
            ops: bounded_symbolic_items(ops_len),
            max_stack: kani::any(),
        }
    }
}

impl kani::Arbitrary for PathSegment {
    fn any() -> Self {
        if kani::any::<bool>() {
            Self::Field(SymbolId::new(kani::any::<u32>()))
        } else {
            Self::Index(kani::any::<u32>())
        }
    }
}

impl kani::Arbitrary for SymbolId {
    fn any() -> Self {
        Self::new(kani::any())
    }
}

impl kani::Arbitrary for ConstValue {
    fn any() -> Self {
        Self::I64(kani::any())
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

impl kani::Arbitrary for ExprBranch {
    fn any() -> Self {
        Self {
            condition: ExprIdx::new(kani::any()),
            target: StepIdx::new(kani::any()),
        }
    }
}

impl kani::Arbitrary for SlotBranch {
    fn any() -> Self {
        Self {
            condition: SlotIdx::new(kani::any()),
            target: StepIdx::new(kani::any()),
        }
    }
}

impl kani::Arbitrary for CompiledNodeKind {
    fn any() -> Self {
        let choice: u8 = kani::any();
        match choice {
            0..=8 => arbitrary_node_group_0(choice),
            9..=17 => arbitrary_node_group_1(choice),
            18..=26 => arbitrary_node_group_2(choice),
            _ => arbitrary_node_group_3(choice),
        }
    }
}

impl kani::Arbitrary for CompiledNode {
    fn any() -> Self {
        Self {
            id: StepIdx::new(kani::any()),
            output: kani::any(),
            routing: kani::any(),
            kind: kani::any::<CompiledNodeKind>(),
        }
    }
}

impl kani::Arbitrary for ResourceContract {
    fn any() -> Self {
        Self {
            max_steps: kani::any(),
            max_slots: kani::any(),
            max_constants: kani::any(),
            max_accessors: kani::any(),
            max_expressions: kani::any(),
            max_expr_stack: kani::any(),
            max_step_budget_per_tick: kani::any(),
            max_transitions_per_tick: kani::any(),
            max_input_bytes: kani::any(),
            max_output_bytes: kani::any(),
            max_blob_bytes: kani::any(),
            max_ipc_payload_bytes: kani::any(),
            max_retry_attempts: kani::any(),
            max_fanout: kani::any(),
            max_collect_items: kani::any(),
            max_queue_depth: kani::any(),
            max_journal_batch_bytes: kani::any(),
            allows_secret_results: kani::any(),
        }
    }
}

impl kani::Arbitrary for WorkflowParts {
    fn any() -> Self {
        let (node_count, nodes) = bounded_workflow_nodes();
        Self {
            name: Box::from("kani_workflow"),
            digest: WorkflowDigest::from_bytes(kani::any::<[u8; 32]>()),
            semantic_source_identity: kani::any(),
            nodes,
            expressions: bounded_workflow_expressions(),
            accessors: bounded_accessors(),
            constants: bounded_workflow_constants(),
            slot_count: kani::any::<u16>(),
            symbols_count: kani::any::<u32>(),
            entry: bounded_workflow_entry(node_count),
            resource_contract: kani::any::<ResourceContract>(),
            step_names: bounded_workflow_step_names(),
            input_slots: bounded_workflow_input_slots(),
        }
    }
}

impl kani::Arbitrary for RunFrame {
    fn any() -> Self {
        let step_count: u16 = kani::any();
        kani::assume(step_count > 0);
        kani::assume(step_count <= 8);
        let slot_count: u16 = kani::any();
        kani::assume(slot_count <= 8);
        let first_step: u16 = kani::any();
        kani::assume(first_step < step_count);
        let run_id = crate::ids::RunId::new(kani::any::<u64>());
        let first_step_idx = StepIdx::new(first_step);
        // With valid assumes, RunFrame::new always succeeds (step_count > 0, first_step < step_count).
        // The Ok variant is guaranteed; Err is unreachable under these assumes.
        match RunFrame::new(run_id, first_step_idx, step_count, slot_count) {
            Ok(frame) => frame,
            Err(_) => {
                kani::assert(false, "valid symbolic frame construction must succeed");
                std::process::abort()
            }
        }
    }
}

include!("kani_workflow_collections.rs");

// -------------------------------------------------------------------------
// kani::Arbitrary for Taint and SlotValue (needed by EngineSignal harness)
// -------------------------------------------------------------------------

impl kani::Arbitrary for Taint {
    fn any() -> Self {
        match kani::any::<u8>() % 5 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            2 => Taint::Secret,
            3 => Taint::Random,
            _ => Taint::TimeDependent,
        }
    }
}

impl kani::Arbitrary for SlotValue {
    fn any() -> Self {
        match kani::any::<u8>() % 8 {
            0 => SlotValue::Null,
            1 => SlotValue::Bool(kani::any()),
            2 => SlotValue::I64(kani::any()),
            3 => SlotValue::F64(kani::any()),
            4 => SlotValue::Symbol(SymbolId::new(kani::any())),
            5 => SlotValue::List(ListId::new(kani::any())),
            6 => SlotValue::Object(ObjectId::new(kani::any())),
            _ => SlotValue::Blob(BlobId::new(kani::any())),
        }
    }
}

// -------------------------------------------------------------------------
// kani::Arbitrary for action system types
// -------------------------------------------------------------------------

include!("kani_workflow_action_arbitrary.rs");

fn arbitrary_optional_step() -> Option<StepIdx> {
    kani::any::<bool>().then(|| StepIdx::new(kani::any()))
}

fn arbitrary_optional_slot() -> Option<SlotIdx> {
    kani::any::<bool>().then(|| SlotIdx::new(kani::any()))
}

include!("kani_workflow_node_generators_0.rs");
include!("kani_workflow_node_generators_1.rs");
include!("kani_workflow_node_dispatch.rs");

include!("kani_workflow_parts_generators.rs");
