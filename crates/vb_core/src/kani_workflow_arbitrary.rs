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
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprBranch, ExprOp, ExprProgram, PathSegment,
    ResourceContract, SlotBranch, WorkflowParts,
};

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
        let mut ops: Vec<ExprOp> = Vec::with_capacity(usize::from(ops_len));
        let mut i = 0u8;
        while i < ops_len {
            ops.push(kani::any::<ExprOp>());
            i += 1;
        }
        let max_stack: u8 = kani::any();
        Self {
            ops: ops.into_boxed_slice(),
            max_stack,
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
            path: bounded_accessor_path(),
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
        match kani::any::<u8>() {
            0 => Self::Nop,
            1 => Self::SetConst {
                value: ConstIdx::new(kani::any()),
            },
            2 => Self::Copy {
                source: SlotIdx::new(kani::any()),
            },
            3 => Self::EvalExpr {
                expr: ExprIdx::new(kani::any()),
            },
            4 => Self::BuildObject {
                fields: Box::new([]),
            },
            5 => Self::BuildList {
                items: Box::new([]),
            },
            6 => Self::Do {
                action: ActionId::new(kani::any()),
                input: SlotIdx::new(kani::any()),
            },
            7 => Self::Choose {
                branches: bounded_expr_branches(),
                otherwise: if kani::any::<bool>() {
                    Some(StepIdx::new(kani::any()))
                } else {
                    None
                },
            },
            8 => Self::ChooseSlot {
                branches: bounded_slot_branches(),
                otherwise: if kani::any::<bool>() {
                    Some(StepIdx::new(kani::any()))
                } else {
                    None
                },
            },
            9 => Self::ForEachStart {
                input: SlotIdx::new(kani::any()),
                item_slot: SlotIdx::new(kani::any()),
                limit: kani::any(),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            10 => Self::ForEachNext {
                iterator_slot: SlotIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            11 => Self::ForEachJoin {
                output: SlotIdx::new(kani::any()),
            },
            12 => Self::TogetherStart {
                branches: bounded_step_indices(),
                join: StepIdx::new(kani::any()),
            },
            13 => Self::TogetherBranch {
                branch: kani::any(),
                entry: StepIdx::new(kani::any()),
                join: StepIdx::new(kani::any()),
                accumulator: SlotIdx::new(kani::any()),
            },
            14 => Self::TogetherJoin {
                branch_count: kani::any(),
                accumulator: SlotIdx::new(kani::any()),
            },
            15 => Self::CollectStart {
                source: SlotIdx::new(kani::any()),
                limit: kani::any(),
                page_size: kani::any(),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            16 => Self::CollectPage {
                collector_slot: SlotIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            17 => Self::CollectNext {
                collector_slot: SlotIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            18 => Self::CollectFinish {
                collector_slot: SlotIdx::new(kani::any()),
            },
            19 => Self::ReduceStart {
                input: SlotIdx::new(kani::any()),
                accumulator: SlotIdx::new(kani::any()),
                initial: ConstIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            20 => Self::ReduceNext {
                iterator_slot: SlotIdx::new(kani::any()),
                accumulator: SlotIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            21 => Self::ReduceFinish {
                accumulator: SlotIdx::new(kani::any()),
            },
            22 => Self::RepeatStart {
                max_attempts: kani::any(),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            23 => Self::RepeatAttempt {
                attempt_slot: SlotIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            24 => Self::RepeatCheck {
                attempt_slot: SlotIdx::new(kani::any()),
                done: StepIdx::new(kani::any()),
            },
            25 => Self::RepeatFinish {
                result: SlotIdx::new(kani::any()),
            },
            26 => Self::WaitUntil {
                deadline_slot: SlotIdx::new(kani::any()),
            },
            27 => Self::WaitEvent {
                event: SlotIdx::new(kani::any()),
                timeout_slot: if kani::any::<bool>() {
                    Some(SlotIdx::new(kani::any()))
                } else {
                    None
                },
            },
            28 => Self::Ask {
                prompt: SlotIdx::new(kani::any()),
                timeout_slot: if kani::any::<bool>() {
                    Some(SlotIdx::new(kani::any()))
                } else {
                    None
                },
            },
            29 => Self::AskResume {
                answer: SlotIdx::new(kani::any()),
            },
            30 => Self::RetryCheck {
                policy_slot: SlotIdx::new(kani::any()),
                body: StepIdx::new(kani::any()),
                exhausted: StepIdx::new(kani::any()),
            },
            31 => Self::ErrorHandler {
                body: StepIdx::new(kani::any()),
                handler: StepIdx::new(kani::any()),
                error_slot: if kani::any::<bool>() {
                    Some(SlotIdx::new(kani::any()))
                } else {
                    None
                },
            },
            32 => Self::Jump {
                target: StepIdx::new(kani::any()),
            },
            _ => Self::Finish {
                result: SlotIdx::new(kani::any()),
            },
        }
    }
}

impl kani::Arbitrary for CompiledNode {
    fn any() -> Self {
        let id = StepIdx::new(kani::any());
        Self {
            id,
            output: if kani::any::<bool>() {
                Some(SlotIdx::new(kani::any()))
            } else {
                None
            },
            next: if kani::any::<bool>() {
                Some(StepIdx::new(kani::any()))
            } else {
                None
            },
            on_error: if kani::any::<bool>() {
                Some(StepIdx::new(kani::any()))
            } else {
                None
            },
            error_slot: if kani::any::<bool>() {
                Some(SlotIdx::new(kani::any()))
            } else {
                None
            },
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
        let node_count: u8 = kani::any();
        kani::assume(node_count <= 8);
        let mut nodes: Vec<CompiledNode> = Vec::with_capacity(usize::from(node_count));
        let mut i = 0u8;
        while i < node_count {
            nodes.push(kani::any::<CompiledNode>());
            i += 1;
        }
        let expr_count: u8 = kani::any();
        kani::assume(expr_count <= 4);
        let mut expressions: Vec<ExprProgram> = Vec::with_capacity(usize::from(expr_count));
        i = 0;
        while i < expr_count {
            expressions.push(kani::any::<ExprProgram>());
            i += 1;
        }
        let const_count: u8 = kani::any();
        kani::assume(const_count <= 4);
        let mut constants: Vec<ConstValue> = Vec::with_capacity(usize::from(const_count));
        i = 0;
        while i < const_count {
            constants.push(kani::any::<ConstValue>());
            i += 1;
        }
        let step_name_count: u8 = kani::any();
        kani::assume(step_name_count <= 4);
        let mut step_names: Vec<Box<str>> = Vec::with_capacity(usize::from(step_name_count));
        i = 0;
        while i < step_name_count {
            step_names.push(kani_step_name(i));
            i += 1;
        }
        Self {
            name: Box::from("kani_workflow"),
            digest: WorkflowDigest::from_bytes(kani::any::<[u8; 32]>()),
            nodes: nodes.into_boxed_slice(),
            expressions: expressions.into_boxed_slice(),
            accessors: bounded_accessors(),
            constants: constants.into_boxed_slice(),
            slot_count: kani::any::<u16>(),
            symbols_count: kani::any::<u32>(),
            entry: {
                let entry_idx: u8 = kani::any();
                let bounded_idx = if node_count == 0 {
                    0
                } else {
                    entry_idx % node_count
                };
                StepIdx::new(bounded_idx as u16)
            },
            resource_contract: kani::any::<ResourceContract>(),
            step_names: step_names.into_boxed_slice(),
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
        // Under kani::assume constraints (step_count > 0, first_step < step_count),
        // RunFrame::new always succeeds. The Err branch is provably dead.
        let frame = match RunFrame::new(run_id, first_step_idx, step_count, slot_count) {
            Ok(f) => f,
            Err(_) => {
                // Provably unreachable: assumes guarantee step_count > 0 and
                // first_step < step_count. Minimal fallback frame, then loop.
                let fallback_id = crate::ids::RunId::new(0);
                if let Ok(f) = RunFrame::new(fallback_id, StepIdx::new(0), 1, 1) {
                    f
                } else {
                    loop {}
                }
            }
        };
        frame
    }
}

fn kani_step_name(index: u8) -> Box<str> {
    match index {
        0 => Box::from("step_0"),
        1 => Box::from("step_1"),
        2 => Box::from("step_2"),
        _ => Box::from("step_3"),
    }
}

fn bounded_accessor_path() -> Box<[PathSegment]> {
    let len: u8 = kani::any();
    kani::assume(len <= 16);
    match len {
        0 => Box::new([]),
        1 => Box::new([kani::any::<PathSegment>()]),
        2 => Box::new([kani::any::<PathSegment>(), kani::any::<PathSegment>()]),
        _ => {
            let mut path = Vec::with_capacity(usize::from(len));
            let mut i = 0u8;
            while i < len {
                path.push(kani::any::<PathSegment>());
                i = match i.checked_add(1) { Some(n) => n, None => break };
            }
            path.into_boxed_slice()
        },
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

fn bounded_expr_branches() -> Box<[ExprBranch]> {
    match bounded_len_2() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<ExprBranch>()]),
        _ => Box::new([kani::any(), kani::any()]),
    }
}

fn bounded_slot_branches() -> Box<[SlotBranch]> {
    match bounded_len_2() {
        0 => Box::new([]),
        1 => Box::new([kani::any::<SlotBranch>()]),
        _ => Box::new([kani::any(), kani::any()]),
    }
}

fn bounded_step_indices() -> Box<[StepIdx]> {
    match bounded_len_2() {
        0 => Box::new([]),
        1 => Box::new([StepIdx::new(kani::any())]),
        _ => Box::new([StepIdx::new(kani::any()), StepIdx::new(kani::any())]),
    }
}

fn bounded_len_3() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= 3);
    len
}

fn bounded_len_2() -> u8 {
    let len: u8 = kani::any();
    kani::assume(len <= 2);
    len
}

// -------------------------------------------------------------------------
// kani::Arbitrary for Taint and SlotValue (needed by EngineSignal harness)
// -------------------------------------------------------------------------

impl kani::Arbitrary for Taint {
    fn any() -> Self {
        match kani::any::<u8>() % 5 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            2 => Taint::Secret,
            3 => Taint::Secret,
            _ => Taint::Secret,
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

impl kani::Arbitrary for Idempotency {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => Idempotency::DeterministicPure,
            1 => Idempotency::IdempotentExternal,
            _ => Idempotency::AtLeastOnceExternal,
        }
    }
}

impl kani::Arbitrary for SideEffect {
    fn any() -> Self {
        match kani::any::<u8>() % 5 {
            0 => SideEffect::Pure,
            1 => SideEffect::LocalWrite,
            2 => SideEffect::ExternalWrite,
            3 => SideEffect::LocalWrite,
            _ => SideEffect::LocalWrite,
        }
    }
}

impl kani::Arbitrary for RetrySafety {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => RetrySafety::Idempotent,
            1 => RetrySafety::RequiresIdempotencyKey,
            _ => RetrySafety::NotRetrySafe,
        }
    }
}

impl kani::Arbitrary for Capability {
    fn any() -> Self {
        Capability::new(
            // Bounded name to avoid unbounded symbolic strings
            format!("cap_{}", kani::any::<u8>().wrapping_rem(16)).into_boxed_str(),
            kani::any(),
        )
    }
}

impl kani::Arbitrary for ActionContract {
    fn any() -> Self {
        // Bounded capability count (0..4) to keep symbolic state space manageable
        let cap_count = kani::any::<u8>() % 4;
        let mut caps = Vec::with_capacity(cap_count as usize);
        let mut i = 0u8;
        while i < cap_count {
            caps.push(kani::any::<Capability>());
            i = match i.checked_add(1) {
                Some(n) => n,
                None => break,
            };
        }
        ActionContract {
            id: kani::any(),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: kani::any(),
            output_slot_count: kani::any(),
            max_input_bytes: kani::any(),
            max_output_bytes: kani::any(),
            timeout_ms: kani::any(),
            idempotency: kani::any(),
            side_effect: kani::any(),
            retry_safety: kani::any(),
            required_capabilities: caps.into_boxed_slice(),
        }
    }
}
