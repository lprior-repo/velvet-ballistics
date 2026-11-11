use super::*;

budget_case!(
    kani_harness_whole_workflow_budget_compute_two_wait_event_nop,
    [
        constructors::wait_event(0, fields(2), bounded_slot(), maybe_slot()),
        constructors::nop(1, fields(2))
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_wait_event_nop",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_wait_event_do,
    [
        constructors::wait_event(0, fields(2), bounded_slot(), maybe_slot()),
        constructors::do_node(1, fields(2), ActionId::new(kani::any()), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_wait_event_do",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_wait_event_wait_until,
    [
        constructors::wait_event(0, fields(2), bounded_slot(), maybe_slot()),
        constructors::wait_until(1, fields(2), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_wait_event_wait_until",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_wait_event_wait_event,
    [
        constructors::wait_event(0, fields(2), bounded_slot(), maybe_slot()),
        constructors::wait_event(1, fields(2), bounded_slot(), maybe_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_wait_event_wait_event",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_wait_event_ask,
    [
        constructors::wait_event(0, fields(2), bounded_slot(), maybe_slot()),
        constructors::ask(1, fields(2), bounded_slot(), maybe_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_wait_event_ask",
    []
);
budget_case!(
    kani_harness_whole_workflow_budget_compute_two_wait_event_finish,
    [
        constructors::wait_event(0, fields(2), bounded_slot(), maybe_slot()),
        constructors::finish(1, fields(2), bounded_slot())
    ],
    bounded_two_node_step(),
    is_ok,
    "partition returned: two_wait_event_finish",
    []
);
