use crate::budget::WholeWorkflowBudget;
use crate::ids::{ActionId, StepIdx};
use crate::workflow::{CompiledNodeKind, ResourceContract};

use super::super::{bounded_slot, bounded_two_node_step, focused_slice_domain, maybe_slot};
use super::sampling::fields;
use super::{constructors, domain};

macro_rules! original_cover {
    (empty, $nodes:ident, $result:ident) => {
        kani::cover!($nodes.is_empty(), "budget generator covers empty workflow");
    };
    (one, $nodes:ident, $result:ident) => {
        kani::cover!(
            $nodes.len() == 1,
            "budget generator covers one-node workflow"
        );
    };
    (two, $nodes:ident, $result:ident) => {
        kani::cover!(
            $nodes.len() == 2,
            "budget generator covers two-node workflow"
        );
    };
    (nop, $nodes:ident, $result:ident) => {
        kani::cover!(
            $nodes
                .iter()
                .any(|node| matches!(node.kind, CompiledNodeKind::Nop)),
            "budget generator covers Nop nodes"
        );
    };
    (do_node, $nodes:ident, $result:ident) => {
        kani::cover!(
            $nodes
                .iter()
                .any(|node| matches!(node.kind, CompiledNodeKind::Do { .. })),
            "budget generator covers Do nodes"
        );
    };
    (wait_until, $nodes:ident, $result:ident) => {
        kani::cover!(
            $nodes
                .iter()
                .any(|node| matches!(node.kind, CompiledNodeKind::WaitUntil { .. })),
            "budget generator covers WaitUntil nodes"
        );
    };
    (finish, $nodes:ident, $result:ident) => {
        kani::cover!(
            $nodes
                .iter()
                .any(|node| matches!(node.kind, CompiledNodeKind::Finish { .. })),
            "budget generator covers Finish nodes"
        );
    };
    (ok, $nodes:ident, $result:ident) => {
        kani::cover!($result.is_ok(), "compute returns Ok for this input");
    };
    (err, $nodes:ident, $result:ident) => {
        kani::cover!($result.is_err(), "compute returns Err for this input");
    };
}

macro_rules! budget_case {
    ($name:ident, $nodes:expr, $entry:expr, $outcome:ident, $label:literal, [$($cover:ident),*]) => {
        #[kani::proof]
        #[kani::unwind(6)]
        fn $name() {
            let contract: ResourceContract = kani::any();
            let nodes = $nodes;
            let entry = $entry;
            assert!(domain::original_projection(&nodes, entry));
            kani::assume(focused_slice_domain(&nodes, entry));
            let result = WholeWorkflowBudget::compute_budget_local(&nodes, entry, &contract);
            $(original_cover!($cover, nodes, result);)*
            kani::cover!(result.$outcome(), $label);
        }
    };
}

budget_case!(
    kani_harness_whole_workflow_budget_compute_empty,
    [],
    StepIdx::new(0),
    is_err,
    "partition returned: empty",
    [empty, err]
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_one_nop,
    [constructors::nop(0, fields(1))],
    StepIdx::new(0),
    is_ok,
    "partition returned: one_nop",
    [one, nop, ok]
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_one_do,
    [constructors::do_node(
        0,
        fields(1),
        ActionId::new(kani::any()),
        bounded_slot()
    )],
    StepIdx::new(0),
    is_ok,
    "partition returned: one_do",
    [do_node]
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_one_wait_until,
    [constructors::wait_until(0, fields(1), bounded_slot())],
    StepIdx::new(0),
    is_ok,
    "partition returned: one_wait_until",
    [wait_until]
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_one_wait_event,
    [constructors::wait_event(
        0,
        fields(1),
        bounded_slot(),
        maybe_slot()
    )],
    StepIdx::new(0),
    is_ok,
    "partition returned: one_wait_event",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_one_ask,
    [constructors::ask(
        0,
        fields(1),
        bounded_slot(),
        maybe_slot()
    )],
    StepIdx::new(0),
    is_ok,
    "partition returned: one_ask",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_one_finish,
    [constructors::finish(0, fields(1), bounded_slot())],
    StepIdx::new(0),
    is_ok,
    "partition returned: one_finish",
    [finish]
);
#[path = "cases/two_ask.rs"]
mod two_ask;
#[path = "cases/two_do.rs"]
mod two_do;
#[path = "cases/two_finish.rs"]
mod two_finish;
#[path = "cases/two_nop.rs"]
mod two_nop;
#[path = "cases/two_wait_event.rs"]
mod two_wait_event;
#[path = "cases/two_wait_until.rs"]
mod two_wait_until;
