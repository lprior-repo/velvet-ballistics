// =====================================================================
// CollectNext: errors on uninitialized collector slot
// =====================================================================

#[test]
fn execute_collect_next_errors_on_uninitialized_collector() -> Result<(), String> {
    let start = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::CollectStart {
            source: SlotIdx::new(1),
            limit: 2.into(),
            page_size: 1,
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let node1 = nop_forward(1, 2);
    let node2 = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::CollectNext {
            collector_slot: SlotIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
        None,
        None,
    );
    let node3 = finish_node(3, 0);
    let wf = make_workflow_with_constants(
        vec![start, node1, node2, node3],
        8,
        Box::from([vb_core::value::ConstValue::I64(42)]),
        &[1],
    );
    let mut run = make_run(8, 4);
    run.set_pc(StepIdx::new(2))
        .map_err(|error| format!("{error:?}"))?;
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = wf
        .node(StepIdx::new(2))
        .ok_or("missing CollectNext node 2".to_string())?;
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
                vb_core::EngineError::SlotUninitialized { slot }
            )) if slot == SlotIdx::new(0)
        ),
        "expected error for uninitialized collector, got {result:?}"
    );
    Ok(())
}

// =====================================================================
// CollectFinish: errors on uninitialized collector slot
// =====================================================================

#[test]
fn execute_collect_finish_errors_on_uninitialized_collector() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::CollectFinish {
            collector_slot: SlotIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let wf = make_workflow(vec![node0], 4, &[0]);
    let mut run = make_run(4, 1);
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

// =====================================================================
// ReduceStart: errors on uninitialized input slot
// =====================================================================

#[test]
fn execute_reduce_start_errors_on_uninitialized_input() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ReduceStart {
            limit: 65_535,
            input: SlotIdx::new(5),
            accumulator: SlotIdx::new(6),
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        },
        Some(SlotIdx::new(7)),
        None,
    );
    let node1 = finish_node(1, 0);
    let node2 = finish_node(2, 0);
    let constants: Box<[vb_core::value::ConstValue]> =
        Box::from([vb_core::value::ConstValue::I64(0)]);
    let wf = make_workflow_with_constants(vec![node0, node1, node2], 8, constants, &[0, 5]);
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
// ReduceNext: errors on uninitialized iterator slot
// =====================================================================

#[test]
fn execute_reduce_next_errors_on_uninitialized_iterator() -> Result<(), String> {
    let start = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ReduceStart {
            limit: 65_535,
            input: SlotIdx::new(2),
            accumulator: SlotIdx::new(1),
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let node1 = nop_forward(1, 2);
    let node2 = vb_core::workflow::node(
        StepIdx::new(2),
        CompiledNodeKind::ReduceNext {
            iterator_slot: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(3),
        },
        Some(SlotIdx::new(3)),
        None,
    );
    let node3 = finish_node(3, 1);
    let wf = make_workflow_with_constants(
        vec![start, node1, node2, node3],
        8,
        Box::from([vb_core::value::ConstValue::I64(42)]),
        &[2],
    );
    let mut run = make_run(8, 4);
    run.set_pc(StepIdx::new(2))
        .map_err(|error| format!("{error:?}"))?;
    let mut store = ValueStore::new();
    let mut cs = CollectStates::new();
    let n = wf
        .node(StepIdx::new(2))
        .ok_or("missing ReduceNext node 2".to_string())?;
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
                vb_core::EngineError::SlotUninitialized { slot }
            )) if slot == SlotIdx::new(0)
        ),
        "expected error for uninitialized iterator, got {result:?}"
    );
    Ok(())
}

// =====================================================================
// ReduceFinish: errors on missing next step
// =====================================================================

#[test]
fn execute_reduce_finish_errors_on_missing_next_step() {
    let node0 = vb_core::workflow::node(
        StepIdx::new(0),
        CompiledNodeKind::ReduceFinish {
            accumulator: SlotIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        None,
    );
    let wf = make_workflow(vec![node0], 4, &[0]);
    let mut run = make_run(4, 1);
    if let Err(e) = run.write_slot(SlotIdx::new(0), SlotValue::I64(42)) {
        let msg = format!("write_slot failed: {e}");
        panic!("{msg}");
    }
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
                vb_core::EngineError::MissingNextStep { .. }
            ))
        ),
        "expected error for missing next step, got {result:?}"
    );
}
