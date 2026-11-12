// =====================================================================
// Fallback dispatch: Nop falls through to step_once
// =====================================================================

#[test]
fn execute_nop_returns_continue_or_budget_exhausted() {
    // Nop at 0 needs next pointing forward. Use Finish at 1 as sink.
    let node0 = nop_forward(0, 1);
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 2);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match result {
        Ok(RuntimeSignal::Continue) => {}
        other => {
            let msg = format!("expected Continue, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Fallback dispatch: Jump falls through to step_once
// =====================================================================

#[test]
fn execute_jump_falls_through_to_step_once() {
    // Jump at 0 targeting 1 (forward edge). Finish at 1 as sink.
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Jump {
            target: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 2);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match result {
        Ok(RuntimeSignal::Continue) => {}
        other => {
            let msg = format!("expected Continue, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Do node: empty contracts fails closed
// =====================================================================

#[test]
fn execute_do_without_contract_rejects_without_ticket() {
    // Do has no kind-specific edges. Single-node workflow with next=None is valid
    // because the validator does not require next for Do (no kind-specific targets).
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Do {
            action: ActionId::new(5),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node], 4);
    let mut run = make_run(4, 2);
    // Input slot must be initialized with clean taint for the no-contract path.
    assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(0)), Ok(()));
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match result {
        Err(RuntimeEngineError::Core(vb_core::EngineError::CapabilityDenied {
            action, ..
        })) => {
            assert_eq!(action, ActionId::new(5));
        }
        other => {
            let msg = format!("expected CapabilityDenied, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Do node: known contract returns AwaitingAction
// =====================================================================

#[test]
fn execute_do_with_known_contract_returns_awaiting_action() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node], 4);
    let mut run = make_run(4, 2);
    assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(10)), Ok(()));
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let contracts: Vec<ActionContract> = vec![
        ActionContract {
            id: ActionId::new(0),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 0,
            output_slot_count: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            timeout_ms: 0,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        },
        ActionContract {
            id: ActionId::new(1),
            name: ActionName::new("test-action").unwrap(),
            input_slot_count: 1,
            output_slot_count: 0,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        },
    ];
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &contracts,
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match result {
        Ok(RuntimeSignal::AwaitingAction(ticket)) => {
            assert_eq!(ticket.action, ActionId::new(1));
            assert_eq!(ticket.run, RunId::new(1));
        }
        other => {
            let msg = format!("expected AwaitingAction, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Do node: unknown contract returns UnknownAction error
// =====================================================================

#[test]
fn execute_do_with_unknown_contract_returns_error() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Do {
            action: ActionId::new(99),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node], 4);
    let mut run = make_run(4, 2);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let contracts: Vec<ActionContract> = vec![ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 0,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }];
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &contracts,
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match result {
        Err(RuntimeEngineError::Action(vb_core::action::ActionError::UnknownAction { action })) => {
            assert_eq!(action, ActionId::new(99));
        }
        other => {
            let msg = format!("expected UnknownAction error, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// Do node: deterministic-pure with secret input triggers taint violation
// =====================================================================

#[test]
fn execute_do_taint_violation_for_deterministic_pure_with_secret_input() {
    let node = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::new(0),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node], 4);
    let mut run = make_run(4, 2);
    assert_eq!(
        run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(1), Taint::Secret),
        Ok(())
    );
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let contracts: Vec<ActionContract> = vec![ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 0,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::new([]),
    }];
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &contracts,
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    match result {
        Err(RuntimeEngineError::TaintViolation { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            let msg = format!("expected TaintViolation, got {other:?}");
            panic!("{msg}");
        }
    }
}

// =====================================================================
// RetryCheck: NEVER policy routes to exhausted (attempt 0 < max 1 -> body)
// =====================================================================

#[test]
fn execute_retry_check_never_policy_uninitialized_routes_to_body() {
    // Uninitialized policy_slot -> attempt=0, NEVER policy: max_attempts=1
    // 0 < 1, so routes to body (not exhausted).
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(0),
            exhausted: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::Continue));
    // NEVER policy: max_attempts=1, attempt=0 < 1, routes to body=step0
    let pc = run.pc();
    assert_eq!(pc, StepIdx::new(0), "expected PC routed to body step 0");
}

// =====================================================================
// RetryCheck: NEVER policy with attempt=1 routes to exhausted
// =====================================================================

#[test]
fn execute_retry_check_never_policy_attempt_one_routes_to_exhausted() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(0),
            exhausted: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 4);
    // Write attempt=1 into the policy slot
    assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(1)), Ok(()));
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::Continue));
    // NEVER policy: max_attempts=1, attempt=1 >= 1, routes to exhausted=step1
    let pc = run.pc();
    assert_eq!(
        pc,
        StepIdx::new(1),
        "expected PC routed to exhausted step 1"
    );
}

// =====================================================================
// RetryCheck: DEFAULT policy routes to body (attempt 1 < max 3)
// =====================================================================

#[test]
fn execute_retry_check_default_policy_routes_to_body() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(0),
            exhausted: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 4);
    assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(1)), Ok(()));
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::DEFAULT,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::Continue));
    // DEFAULT policy: max_attempts=3, attempt=1 < 3, routes to body=step0
    let pc = run.pc();
    assert_eq!(pc, StepIdx::new(0), "expected PC routed to body step 0");
}

// =====================================================================
// RetryCheck: DEFAULT policy with attempt=3 routes to exhausted
// =====================================================================

#[test]
fn execute_retry_check_default_policy_attempt_three_routes_to_exhausted() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RetryCheck {
            policy_slot: SlotIdx::new(0),
            body: StepIdx::new(0),
            exhausted: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 4);
    assert_eq!(run.write_slot(SlotIdx::new(0), SlotValue::I64(3)), Ok(()));
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::DEFAULT,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::Continue));
    // DEFAULT policy: max_attempts=3, attempt=3 >= 3, routes to exhausted=step1
    let pc = run.pc();
    assert_eq!(
        pc,
        StepIdx::new(1),
        "expected PC routed to exhausted step 1"
    );
}

// =====================================================================
// ErrorHandler: routes PC to body step
// =====================================================================

#[test]
fn execute_error_handler_routes_to_body_step() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: None.into(),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let wf = make_workflow(vec![node0, node1, node2], 4);
    let mut run = make_run(4, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::Continue));
    let pc = run.pc();
    assert_eq!(pc, StepIdx::new(1), "expected PC routed to body step 1");
}

#[test]
fn execute_error_handler_with_error_slot_routes_to_body_step() {
    let node0 = vb_core::workflow::node_with_handler(
        StepIdx::new(0),
        CompiledNodeKind::ErrorHandler {
            body: StepIdx::new(1),
            handler: StepIdx::new(2),
            error_slot: Some(SlotIdx::new(3)).into(),
        },
        None,
        None,
        StepIdx::new(2),
        Some(SlotIdx::new(3)),
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let wf = make_workflow(vec![node0, node1, node2], 8);
    let mut run = make_run(8, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert_eq!(result, Ok(RuntimeSignal::Continue));
    let pc = run.pc();
    assert_eq!(pc, StepIdx::new(1), "expected PC routed to body step 1");
}

// =====================================================================
// ForEachStart: errors on uninitialized input slot
// =====================================================================

#[test]
fn execute_for_each_start_errors_on_uninitialized_input() {
    // body=1 (forward, makes node 1 reachable), done=2 (forward)
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(5),
            item_slot: SlotIdx::new(6),
            limit: 10.into(),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let wf = make_workflow(vec![node0, node1, node2], 8);
    let mut run = make_run(8, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::SlotUninitialized { .. }
            ))
        ),
        "expected error for uninitialized input, got {result:?}"
    );
}

// =====================================================================
// ForEachJoin: errors on missing next step
// =====================================================================

#[test]
fn execute_for_each_join_errors_on_missing_next_step() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ForEachJoin {
            output: SlotIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let wf = make_workflow(vec![node0], 4);
    let mut run = make_run(4, 1);
    let mut store = ValueStore::new();
    let list_id = match store.insert_list(Vec::<SlotValue>::new().into_boxed_slice()) {
        Ok(id) => id,
        Err(e) => {
            let msg = format!("insert_list failed: {e}");
            panic!("{msg}");
        }
    };
    if let Err(e) = run.write_slot(SlotIdx::new(0), SlotValue::List(list_id)) {
        let msg = format!("write_slot failed: {e}");
        panic!("{msg}");
    }
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::MissingNextStep { .. }
            ))
        ),
        "expected error for missing next step, got {result:?}"
    );
}

// =====================================================================
// ForEachNext: errors on uninitialized iterator slot
// =====================================================================

#[test]
fn execute_for_each_next_errors_on_uninitialized_iterator() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ForEachNext {
            iterator_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let wf = make_workflow(vec![node0, node1, node2], 4);
    let mut run = make_run(4, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::SlotUninitialized { .. }
            ))
        ),
        "expected error for uninitialized iterator, got {result:?}"
    );
}

// =====================================================================
// TogetherStart: empty branches should not panic
// =====================================================================

#[test]
fn execute_together_start_empty_branches_no_panic() {
    // join=1 must be forward
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::TogetherStart {
            branches: Box::from([]),
            join: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 4);
    let mut run = make_run(4, 2);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    // This catch_unwind match is a panic-safety guard, not a fallible-result
    // assertion. It verifies that execute_node_full itself does not panic;
    // the inner RuntimeEngineResult is intentionally not asserted here.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = execute_node_full(
            &wf,
            &mut run,
            &mut store,
            n,
            &[],
            RetryPolicy::NEVER,
            &mut cs,
            &CapabilitySet::empty(),
            1,
        );
    }));
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

// =====================================================================
// TogetherJoin: errors on missing next step
// =====================================================================

#[test]
fn execute_together_join_errors_on_missing_next_step() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::TogetherJoin {
            branch_count: 2.into(),
            accumulator: SlotIdx::new(1),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let wf = make_workflow(vec![node0], 4);
    let mut run = make_run(4, 1);
    let mut store = ValueStore::new();
    let list_id = match store.insert_list(Vec::<SlotValue>::new().into_boxed_slice()) {
        Ok(id) => id,
        Err(e) => {
            let msg = format!("insert_list failed: {e}");
            panic!("{msg}");
        }
    };
    if let Err(e) = run.write_slot(SlotIdx::new(1), SlotValue::List(list_id)) {
        let msg = format!("write accumulator failed: {e}");
        panic!("{msg}");
    }
    if let Err(e) = run.write_slot(SlotIdx::new(0), SlotValue::Null) {
        let msg = format!("write output failed: {e}");
        panic!("{msg}");
    }
    if let Err(e) = run.add_parallel_in_flight(2) {
        let msg = format!("add_parallel_in_flight failed: {e}");
        panic!("{msg}");
    }
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::MissingNextStep { .. }
            ))
        ),
        "expected error for missing next step, got {result:?}"
    );
}

// =====================================================================
// CollectStart: errors on uninitialized source slot
// =====================================================================

#[test]
fn execute_collect_start_errors_on_uninitialized_source() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::CollectStart {
            source: SlotIdx::new(5),
            limit: 10.into(),
            page_size: 5,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let wf = make_workflow(vec![node0, node1, node2], 8);
    let mut run = make_run(8, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::SlotUninitialized { .. }
            ))
        ),
        "expected error for uninitialized source, got {result:?}"
    );
}

// =====================================================================
// CollectPage: errors on uninitialized collector slot
// =====================================================================

#[test]
fn execute_collect_page_errors_on_uninitialized_collector() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::CollectPage {
            collector_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let wf = make_workflow(vec![node0, node1, node2], 8);
    let mut run = make_run(8, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = match wf.node(StepIdx::ZERO) {
        Some(n) => n,
        None => return,
    };
    let result = execute_node_full(
        &wf,
        &mut run,
        &mut store,
        n,
        &[],
        RetryPolicy::NEVER,
        &mut cs,
        &CapabilitySet::empty(),
        1,
    );
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::SlotUninitialized { .. }
            ))
        ),
        "expected error for uninitialized collector, got {result:?}"
    );
}

