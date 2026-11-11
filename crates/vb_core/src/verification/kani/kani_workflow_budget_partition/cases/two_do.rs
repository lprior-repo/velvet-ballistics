use super::*;

budget_case!(
    kani_harness_whole_workflow_budget_compute_two_do_nop,
    [
        constructors::do_node(0, fields(2), ActionId::new(kani::any()), bounded_slot()),
        constructors::nop(1, fields(2))
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_do_nop",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_do_do,
    [
        constructors::do_node(0, fields(2), ActionId::new(kani::any()), bounded_slot()),
        constructors::do_node(1, fields(2), ActionId::new(kani::any()), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_do_do",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_do_wait_until,
    [
        constructors::do_node(0, fields(2), ActionId::new(kani::any()), bounded_slot()),
        constructors::wait_until(1, fields(2), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_do_wait_until",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_do_wait_event,
    [
        constructors::do_node(0, fields(2), ActionId::new(kani::any()), bounded_slot()),
        constructors::wait_event(1, fields(2), bounded_slot(), maybe_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_do_wait_event",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_do_ask,
    [
        constructors::do_node(0, fields(2), ActionId::new(kani::any()), bounded_slot()),
        constructors::ask(1, fields(2), bounded_slot(), maybe_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_do_ask",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_do_finish,
    [
        constructors::do_node(0, fields(2), ActionId::new(kani::any()), bounded_slot()),
        constructors::finish(1, fields(2), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_do_finish",
    []
);
