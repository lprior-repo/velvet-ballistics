use vb_core::ids::FanoutLimit;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx};

pub(super) const SOURCE: &[u8] = b"version: velvet-ballistics/v1\nname: nested\nwhen: { manual: {} }\nsteps:\n  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 1\n      items: 1\n      steps:\n        - id: inner\n          collect:\n            variable: inner_page\n            source: \"1\"\n            pages: 1\n            items: 1\n            steps:\n              - id: leaf\n                set: { output: picked, value: \"7\" }\n  - id: done\n    finish: { result: 0 }\n";

fn kind_collect_start_outer() -> CompiledNodeKind {
    CompiledNodeKind::CollectStart {
        source: SlotIdx::new(0),
        limit: FanoutLimit::new(1),
        page_size: 1,
        body: StepIdx::new(1),
        done: StepIdx::new(7),
    }
}

fn kind_collect_start_inner() -> CompiledNodeKind {
    CompiledNodeKind::CollectStart {
        source: SlotIdx::new(1),
        limit: FanoutLimit::new(1),
        page_size: 1,
        body: StepIdx::new(2),
        done: StepIdx::new(6),
    }
}

fn kind_set_const() -> CompiledNodeKind {
    CompiledNodeKind::SetConst {
        value: ConstIdx::new(0),
    }
}

fn kind_collect_next_inner() -> CompiledNodeKind {
    CompiledNodeKind::CollectNext {
        collector_slot: SlotIdx::new(1),
        body: StepIdx::new(2),
        done: StepIdx::new(6),
    }
}

fn kind_collect_finish_inner() -> CompiledNodeKind {
    CompiledNodeKind::CollectFinish {
        collector_slot: SlotIdx::new(1),
    }
}

fn kind_collect_next_outer() -> CompiledNodeKind {
    CompiledNodeKind::CollectNext {
        collector_slot: SlotIdx::new(0),
        body: StepIdx::new(1),
        done: StepIdx::new(7),
    }
}

fn kind_collect_finish_outer() -> CompiledNodeKind {
    CompiledNodeKind::CollectFinish {
        collector_slot: SlotIdx::new(0),
    }
}

fn kind_finish() -> CompiledNodeKind {
    CompiledNodeKind::Finish {
        result: SlotIdx::new(2),
    }
}

fn expected_kinds() -> [CompiledNodeKind; 8] {
    [
        kind_collect_start_outer(),
        kind_collect_start_inner(),
        kind_set_const(),
        kind_collect_next_inner(),
        kind_collect_finish_inner(),
        kind_collect_next_outer(),
        kind_collect_finish_outer(),
        kind_finish(),
    ]
}

pub(super) fn assert_kind_counts(nodes: &[CompiledNode]) {
    let collect_start = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::CollectStart { .. }))
        .count();
    let collect_next = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::CollectNext { .. }))
        .count();
    let collect_finish = nodes
        .iter()
        .filter(|n| matches!(n.kind, CompiledNodeKind::CollectFinish { .. }))
        .count();
    assert_eq!(
        collect_start, 2,
        "two CollectStart nodes for outer and inner"
    );
    assert_eq!(collect_next, 2, "two CollectNext nodes for outer and inner");
    assert_eq!(
        collect_finish, 2,
        "two CollectFinish nodes for outer and inner"
    );
}

pub(super) fn assert_kind_order(nodes: &[CompiledNode]) {
    let expected_kinds = expected_kinds();
    let actual_discriminants = nodes
        .iter()
        .map(|n| std::mem::discriminant(&n.kind))
        .collect::<Vec<_>>();
    let expected_discriminants = expected_kinds
        .iter()
        .map(std::mem::discriminant)
        .collect::<Vec<_>>();
    assert_eq!(
        actual_discriminants, expected_discriminants,
        "exact node kind order for outer collect wrapping inner collect"
    );
}

fn collector_slot(nodes: &[CompiledNode], index: usize) -> Option<SlotIdx> {
    nodes.get(index).and_then(|n| match n.kind {
        CompiledNodeKind::CollectNext { collector_slot, .. } => Some(collector_slot),
        _ => None,
    })
}

pub(super) fn assert_collector_slots(nodes: &[CompiledNode]) {
    let inner_collect_slot = collector_slot(nodes, 3);
    let outer_collect_slot = collector_slot(nodes, 5);
    assert_eq!(
        inner_collect_slot,
        Some(vb_core::SlotIdx::new(4)),
        "node 3 must be CollectNext with collector slot 4"
    );
    assert_eq!(
        outer_collect_slot,
        Some(vb_core::SlotIdx::new(2)),
        "node 5 must be CollectNext with collector slot 2"
    );
    assert_ne!(
        inner_collect_slot, outer_collect_slot,
        "inner and outer collect must have different collector slots"
    );
}

fn assert_inner_region(workflow: &CompiledWorkflow) {
    let inner_fields = match workflow.loop_region(StepIdx::new(3)) {
        Some(r) => (
            r.start().get(),
            r.body().get(),
            r.control().get(),
            r.done().get(),
        ),
        None => (0, 0, 0, 0),
    };
    assert_ne!(
        inner_fields,
        (0, 0, 0, 0),
        "inner collect must have loop region"
    );
    assert_eq!(inner_fields.0, 1, "inner region start");
    assert_eq!(inner_fields.1, 2, "inner region body");
    assert_eq!(inner_fields.2, 3, "inner region control");
    assert_eq!(inner_fields.3, 4, "inner region done");
}

fn assert_outer_region(workflow: &CompiledWorkflow) {
    let outer_fields = match workflow.loop_region(StepIdx::new(5)) {
        Some(r) => (
            r.start().get(),
            r.body().get(),
            r.control().get(),
            r.done().get(),
        ),
        None => (0, 0, 0, 0),
    };
    assert_ne!(
        outer_fields,
        (0, 0, 0, 0),
        "outer collect must have loop region"
    );
    assert_eq!(outer_fields.0, 0, "outer region start");
    assert_eq!(outer_fields.1, 1, "outer region body");
    assert_eq!(outer_fields.2, 5, "outer region control");
    assert_eq!(outer_fields.3, 6, "outer region done");
}

pub(super) fn assert_loop_regions(workflow: &CompiledWorkflow) {
    assert_inner_region(workflow);
    assert_outer_region(workflow);
    assert!(
        workflow.loop_region(vb_core::StepIdx::new(2)).is_none(),
        "SetConst node must not be a loop region control"
    );
}
