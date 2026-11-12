// =====================================================================
// RepeatStart: zero max_attempts should not panic
// =====================================================================

#[test]
fn execute_repeat_start_single_attempt_no_panic() -> Result<(), String> {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RepeatStart {
            max_attempts: 1.into(),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let mut node1 = finish_node(1, 0);
    node1.output = NodeOutput::Discard;
    let mut node2 = finish_node(2, 0);
    node2.output = NodeOutput::Discard;
    let wf = make_workflow(vec![node0, node1, node2], 8, &[]);
    let mut run = make_run(8, 4);
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = wf.node(StepIdx::ZERO).ok_or("missing repeat start")?;
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
    assert_eq!(run.pc(), StepIdx::new(1));
    assert_eq!(
        run.read_slot(SlotIdx::new(0)),
        Ok(&SlotValue::I64(4294967296))
    );
    assert_eq!(run.read_taint(SlotIdx::new(0)), Ok(Taint::Clean));
    Ok(())
}

// =====================================================================
// RepeatAttempt: errors on uninitialized attempt slot
// =====================================================================

#[test]
fn execute_repeat_attempt_errors_on_uninitialized_attempt_slot() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot: SlotIdx::new(5),
            body: StepIdx::new(0),
            done: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 8, &[0, 5]);
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
        "expected error for uninitialized attempt slot, got {result:?}"
    );
}

// =====================================================================
// RepeatFinish: errors on uninitialized result slot
// =====================================================================

#[test]
fn execute_repeat_finish_errors_on_uninitialized_result_slot() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RepeatFinish {
            result: SlotIdx::new(5),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let wf = make_workflow(vec![node0], 8, &[5]);
    let mut run = make_run(8, 1);
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
        "expected error for uninitialized result slot, got {result:?}"
    );
}

// =====================================================================
// WaitUntil: errors on uninitialized deadline slot
// =====================================================================

#[test]
fn execute_wait_until_errors_on_uninitialized_deadline() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::new(5),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node0], 8, &[5]);
    let mut run = make_run(8, 1);
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
        "expected error for uninitialized deadline, got {result:?}"
    );
}

// =====================================================================
// WaitEvent: errors on uninitialized event slot
// =====================================================================

#[test]
fn execute_wait_event_errors_on_uninitialized_event() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::new(5),
            timeout_slot: None.into(),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node0], 8, &[5]);
    let mut run = make_run(8, 1);
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
        "expected error for uninitialized event, got {result:?}"
    );
}

// =====================================================================
// Ask: errors on uninitialized prompt slot
// =====================================================================

#[test]
fn execute_ask_errors_on_uninitialized_prompt() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::Ask {
            prompt: SlotIdx::new(5),
            timeout_slot: None.into(),
        },
        None,
        None,
    );
    let wf = make_workflow(vec![node0], 8, &[5]);
    let mut run = make_run(8, 1);
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
        "expected error for uninitialized prompt, got {result:?}"
    );
}

// =====================================================================
// AskResume: errors on uninitialized answer slot
// =====================================================================

#[test]
fn execute_ask_resume_errors_on_uninitialized_answer() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::AskResume {
            answer: SlotIdx::new(5),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let wf = make_workflow(vec![node0], 8, &[5]);
    let mut run = make_run(8, 1);
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
        "expected error for uninitialized answer, got {result:?}"
    );
}

// =====================================================================
// RepeatCheck: routes to done when attempt exceeds max
// =====================================================================

#[test]
fn execute_repeat_check_routes_forward_on_done() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::RepeatCheck {
            attempt_slot: SlotIdx::new(0),
            done: StepIdx::new(1),
        },
        None,
        None,
    );
    let node1 = finish_node(1, 0);
    let wf = make_workflow(vec![node0, node1], 8, &[0]);
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
    // RepeatCheck reads attempt_slot, but since it is uninitialized we expect an error
    assert!(
        matches!(
            result,
            Err(RuntimeEngineError::Core(
                vb_core::EngineError::SlotUninitialized { .. }
            ))
        ),
        "expected error for uninitialized attempt slot, got {result:?}"
    );
}
