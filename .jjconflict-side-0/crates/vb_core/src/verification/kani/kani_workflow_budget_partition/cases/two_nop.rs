use super::*;

budget_case!(
    kani_harness_whole_workflow_budget_compute_two_nop_nop,
    [
        constructors::nop(0, fields(2)),
        constructors::nop(1, fields(2))
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_nop_nop",
    [two]
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_nop_do,
    [
        constructors::nop(0, fields(2)),
        constructors::do_node(1, fields(2), ActionId::new(kani::any()), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_nop_do",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_nop_wait_until,
    [
        constructors::nop(0, fields(2)),
        constructors::wait_until(1, fields(2), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_nop_wait_until",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_nop_wait_event,
    [
        constructors::nop(0, fields(2)),
        constructors::wait_event(1, fields(2), bounded_slot(), maybe_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_nop_wait_event",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_nop_ask,
    [
        constructors::nop(0, fields(2)),
        constructors::ask(1, fields(2), bounded_slot(), maybe_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_nop_ask",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_nop_finish,
    [
        constructors::nop(0, fields(2)),
        constructors::finish(1, fields(2), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_nop_finish",
    []
);
