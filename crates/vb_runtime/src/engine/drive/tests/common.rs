#![forbid(unsafe_code)]

//! Shared behavior-test helpers for the drive loop tests.
//!
//! Each child module under this `tests` directory covers one test family.
//! Helpers are kept here so the per-family files stay under the
//! 300-line drift ceiling while sharing the same workflow/RunFrame
//! construction helpers.

use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, RetryPolicy, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::engine::StepBudget;
use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, SlotBranch, WorkflowParts,
};

pub(crate) fn cn(
    id: u16,
    output: Option<u16>,
    next: Option<u16>,
    kind: CompiledNodeKind,
) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(id),
        output: output.map(SlotIdx::new),
        next: next.map(StepIdx::new),
        on_error: None,
        error_slot: None,
        kind,
    }
}

pub(crate) fn fin(id: u16, result: u16) -> CompiledNode {
    cn(
        id,
        None,
        None,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(result),
        },
    )
}

pub(crate) fn nop(id: u16, nx: u16) -> CompiledNode {
    cn(id, None, Some(nx), CompiledNodeKind::Nop)
}

pub(crate) fn setc(id: u16, cid: u16, out: u16, nx: u16) -> CompiledNode {
    cn(
        id,
        Some(out),
        Some(nx),
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(cid),
        },
    )
}

pub(crate) fn cpy(id: u16, src: u16, out: u16, nx: u16) -> CompiledNode {
    cn(
        id,
        Some(out),
        Some(nx),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(src),
        },
    )
}

pub(crate) fn don(id: u16, action: u16, inp: u16) -> CompiledNode {
    cn(
        id,
        None,
        None,
        CompiledNodeKind::Do {
            action: ActionId::new(action),
            input: SlotIdx::new(inp),
        },
    )
}

pub(crate) fn collect_start(id: u16, source: u16, out: u16, body: u16, done: u16) -> CompiledNode {
    cn(
        id,
        Some(out),
        None,
        CompiledNodeKind::CollectStart {
            source: SlotIdx::new(source),
            limit: 100,
            page_size: 1,
            body: StepIdx::new(body),
            done: StepIdx::new(done),
        },
    )
}

pub(crate) fn cslot(id: u16, branches: Box<[SlotBranch]>, otw: Option<u16>) -> CompiledNode {
    cn(
        id,
        None,
        otw,
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise: otw.map(StepIdx::new),
        },
    )
}

pub(crate) fn askn(id: u16, prompt: u16) -> CompiledNode {
    cn(
        id,
        None,
        None,
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(prompt),
            timeout_slot: None,
        },
    )
}

pub(crate) fn wuntil(id: u16, dl: u16) -> CompiledNode {
    cn(
        id,
        None,
        None,
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(dl),
        },
    )
}

pub(crate) fn errh(id: u16, body: u16, handler: u16, eslot: Option<u16>) -> CompiledNode {
    cn(
        id,
        None,
        None,
        CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(body),
            handler: StepIdx::new(handler),
            error_slot: eslot.map(SlotIdx::new),
        },
    )
}

pub(crate) fn tog(id: u16, branches: Box<[u16]>, join: u16) -> CompiledNode {
    let br: Box<[StepIdx]> = branches.iter().map(|b| StepIdx::new(*b)).collect();
    cn(
        id,
        None,
        None,
        CompiledNodeKind::TogetherStart {
            branches: br,
            join: StepIdx::new(join),
        },
    )
}

pub(crate) fn mkwf(nodes: Vec<CompiledNode>, sc: u16) -> Result<CompiledWorkflow, String> {
    mkwfc(nodes, sc, vec![])
}

pub(crate) fn mkwfc(
    nodes: Vec<CompiledNode>,
    sc: u16,
    consts: Vec<ConstValue>,
) -> Result<CompiledWorkflow, String> {
    let names: Box<[Box<str>]> = (0..nodes.len())
        .map(|i| format!("s{i}").into_boxed_str())
        .collect();
    let parts = WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: consts.into_boxed_slice(),
        slot_count: sc,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: names,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("{e}"))
}

pub(crate) fn mkr(steps: u16, slots: u16) -> Result<RunFrame, String> {
    RunFrame::new(RunId::new(1), StepIdx::new(0), steps, slots).map_err(|e| format!("{e}"))
}

pub(crate) fn dd(
    wf: &CompiledWorkflow,
    r: &mut RunFrame,
    b: &mut StepBudget,
) -> Result<RuntimeSignal, String> {
    let mut store = ValueStore::new();
    let mut ev = EvidenceCollector::new();
    let mut cs = CollectStates::new();
    drive_deterministic_full(
        wf,
        r,
        b,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut ev,
        &mut cs,
        &CapabilitySet::empty(),
    )
    .map_err(|e| format!("{e}"))
}

pub(crate) fn dde(
    wf: &CompiledWorkflow,
    r: &mut RunFrame,
    b: &mut StepBudget,
    ev: &mut EvidenceCollector,
    g: &CapabilitySet,
) -> Result<RuntimeSignal, String> {
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    drive_deterministic_full(
        wf,
        r,
        b,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        ev,
        &mut cs,
        g,
    )
    .map_err(|e| format!("{e}"))
}

pub(crate) fn ws(r: &mut RunFrame, s: u16, v: SlotValue) -> Result<(), String> {
    r.write_slot(SlotIdx::new(s), v).map_err(|e| format!("{e}"))
}

pub(crate) fn gr(name: &str, action: u16) -> CapabilitySet {
    CapabilitySet::from_grants(Box::from([Capability::new(
        name.into(),
        ActionId::new(action),
    )]))
}
