//! Resource budget fuzz target bodies.

const FUZZ_MAX_NODES: usize = 32;

pub fn fuzz_resource_budget(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let first_byte = data.first().copied().unwrap_or(0);
    let constant = match first_byte.wrapping_rem(4) {
        0 => vb_core::ConstValue::I64(i64::from(first_byte)),
        1 => vb_core::ConstValue::Bool(first_byte.is_multiple_of(2)),
        2 => vb_core::ConstValue::Null,
        _ => vb_core::ConstValue::I64(42),
    };
    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(two_node_parts(constant)) else {
        return;
    };
    let budget_value = budget_from_bytes(data);
    let Ok(mut run) = vb_core::RunFrame::new(
        vb_core::RunId::new(3),
        vb_core::StepIdx::ZERO,
        workflow.node_count(),
        workflow.slot_count(),
    ) else {
        return;
    };
    let mut store = vb_core::ValueStore::new();
    let initial_executed = run.executed();
    let result = vb_core::engine::run_until_blocked(
        &workflow,
        &mut run,
        vb_core::StepBudget::new(budget_value),
        &mut store,
    );
    let Ok(signal) = result else {
        return;
    };
    let executed_delta = run.executed().saturating_sub(initial_executed);
    if budget_value == 0 {
        assert_eq!(executed_delta, 0);
        assert_eq!(signal, vb_core::EngineSignal::StepBudgetExhausted);
    }
    assert!(executed_delta <= budget_value);
}

fn two_node_parts(constant: vb_core::ConstValue) -> vb_core::WorkflowParts {
    vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_budget"),
        digest: vb_core::WorkflowDigest::from_bytes([0xC0; 32]),
        nodes: vec![
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(0),
                output: Some(vb_core::SlotIdx::new(0)),
                next: Some(vb_core::StepIdx::new(1)),
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::SetConst {
                    value: vb_core::ConstIdx::new(0),
                },
            },
            vb_core::CompiledNode {
                id: vb_core::StepIdx::new(1),
                output: None,
                next: None,
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::Finish {
                    result: vb_core::SlotIdx::new(0),
                },
            },
        ]
        .into(),
        expressions: vec![].into(),
        accessors: vec![].into(),
        constants: vec![constant].into(),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn budget_from_bytes(data: &[u8]) -> u64 {
    let value = if data.len() >= 9 {
        let mut bytes = [0u8; 8];
        let Some(slice) = data.get(1..9) else {
            return 0;
        };
        let mut src = [0u8; 8];
        let end = slice.len().min(8);
        if end > 0 {
            src[..end].copy_from_slice(&slice[..end]);
        }
        bytes.copy_from_slice(&src);
        u64::from_le_bytes(bytes)
    } else {
        u64::from(data.get(1).copied().unwrap_or(0))
    };
    value.wrapping_rem(1000)
}

pub fn fuzz_budget_compute(data: &[u8]) {
    if data.len() < 3 {
        return;
    }
    let Some(&byte0) = data.first() else {
        return;
    };
    let Some(&byte1) = data.get(1) else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(16))
        .saturating_add(1)
        .min(FUZZ_MAX_NODES);
    let slot_count = u16::from(byte1.wrapping_rem(16)).saturating_add(1);
    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let Some(offset) = i.saturating_add(2).checked_rem(data.len()) else {
            continue;
        };
        let kind_byte = data.get(offset).copied().unwrap_or(0);
        nodes.push(build_fuzz_budget_node(i, kind_byte, node_count, slot_count));
    }
    let contract = vb_core::ResourceContract {
        max_slots: slot_count,
        ..vb_core::ResourceContract::DEFAULT
    };
    let result =
        vb_core::budget::WholeWorkflowBudget::compute(&nodes, vb_core::StepIdx::ZERO, &contract);
    let Ok(budget) = result else {
        return;
    };
    if !nodes.is_empty() {
        assert!(budget.max_total_steps > 0);
    }
    assert!(budget.max_total_slots >= u64::from(slot_count));
    let _ = budget.max_total_steps;
    let _ = budget.max_total_slots;
    let _ = budget.max_fanout;
}

fn build_fuzz_budget_node(
    index: usize,
    kind_byte: u8,
    node_count: usize,
    slot_count: u16,
) -> vb_core::CompiledNode {
    let step_idx = vb_core::StepIdx::new(u16::try_from(index).unwrap_or(u16::MAX));
    let next_step = if index.saturating_add(1) < node_count {
        Some(vb_core::StepIdx::new(
            u16::try_from(index).unwrap_or(0).saturating_add(1),
        ))
    } else {
        None
    };
    let safe_slot = vb_core::SlotIdx::new(slot_count.saturating_sub(1));
    let kind = match kind_byte.wrapping_rem(6) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish { result: safe_slot },
        2 => vb_core::CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        3 => vb_core::CompiledNodeKind::Copy { source: safe_slot },
        4 => foreach_start(index, node_count, safe_slot),
        _ => together_start(index, node_count),
    };
    vb_core::CompiledNode {
        id: step_idx,
        output: Some(safe_slot),
        next: next_step,
        error_slot: None,
        on_error: None,
        kind,
    }
}

fn bounded_next(index: usize, node_count: usize, add: usize) -> vb_core::StepIdx {
    vb_core::StepIdx::new(
        u16::try_from(index.saturating_add(add).min(node_count.saturating_sub(1))).unwrap_or(0),
    )
}

fn foreach_start(
    index: usize,
    node_count: usize,
    safe_slot: vb_core::SlotIdx,
) -> vb_core::CompiledNodeKind {
    vb_core::CompiledNodeKind::ForEachStart {
        input: safe_slot,
        item_slot: safe_slot,
        limit: 5,
        body: bounded_next(index, node_count, 1),
        done: bounded_next(index, node_count, 2),
    }
}

fn together_start(index: usize, node_count: usize) -> vb_core::CompiledNodeKind {
    let branch_idx = bounded_next(index, node_count, 1);
    vb_core::CompiledNodeKind::TogetherStart {
        branches: vec![branch_idx, branch_idx].into_boxed_slice(),
        join: bounded_next(index, node_count, 2),
    }
}
