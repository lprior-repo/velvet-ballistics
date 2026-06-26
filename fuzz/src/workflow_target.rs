//! Workflow compilation, IR, and resource budget fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

const FUZZ_MAX_NODES: usize = 32;
const FUZZ_MAX_ACCESSOR_DEPTH: usize = 16;

pub fn fuzz_compiled_ir(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let digest_before = parts.digest;
        let node_count_before = parts.nodes.len();
        let slot_count = parts.slot_count;
        let result = vb_core::CompiledWorkflow::try_from_parts(parts);
        if let Ok(workflow) = result {
            assert!(
                workflow.node_count() >= 1,
                "compiled workflow must have at least 1 node, got {}",
                workflow.node_count()
            );
            assert_eq!(
                workflow.slot_count(),
                slot_count,
                "workflow slot count must match decoded parts slot count"
            );
            assert_eq!(
                workflow.digest(),
                digest_before,
                "workflow digest must match decoded parts digest"
            );
            assert_eq!(
                usize::from(workflow.node_count()),
                node_count_before,
                "workflow node count must match decoded parts node count"
            );
            for i in 0..workflow.node_count() {
                let step = vb_core::StepIdx::new(i);
                let Some(node) = workflow.node(step) else {
                    continue;
                };
                if let Some(output) = node.output {
                    assert!(
                        output.get() < slot_count,
                        "node {} output slot {} out of bounds (slot_count={})",
                        i,
                        output.get(),
                        slot_count
                    );
                }
                check_node_slots(&node.kind, slot_count, i);
            }
        }
    }
}

fn check_node_slots(kind: &vb_core::CompiledNodeKind, slot_count: u16, node_idx: u16) {
    use vb_core::CompiledNodeKind;
    match kind {
        CompiledNodeKind::Nop | CompiledNodeKind::Jump { .. } => {}
        CompiledNodeKind::SetConst { .. } => {}
        CompiledNodeKind::Copy { source } => {
            assert!(
                source.get() < slot_count,
                "node {} Copy source slot {} out of bounds",
                node_idx,
                source.get()
            );
        }
        CompiledNodeKind::EvalExpr { expr: _ } => {}
        CompiledNodeKind::BuildObject { fields } => {
            for (_, slot) in fields.iter() {
                assert!(
                    slot.get() < slot_count,
                    "node {} BuildObject slot {} out of bounds",
                    node_idx,
                    slot.get()
                );
            }
        }
        CompiledNodeKind::BuildList { items } => {
            for slot in items.iter() {
                assert!(
                    slot.get() < slot_count,
                    "node {} BuildList slot {} out of bounds",
                    node_idx,
                    slot.get()
                );
            }
        }
        CompiledNodeKind::Do { action: _, input } => {
            assert!(
                input.get() < slot_count,
                "node {} Do input slot {} out of bounds",
                node_idx,
                input.get()
            );
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for _branch in branches.iter() {}
            let _ = otherwise;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.iter() {
                assert!(
                    branch.condition.get() < slot_count,
                    "node {} ChooseSlot condition slot {} out of bounds",
                    node_idx,
                    branch.condition.get()
                );
            }
            let _ = otherwise;
        }
        CompiledNodeKind::ForEachStart {
            input, item_slot, ..
        } => {
            assert!(
                input.get() < slot_count,
                "node {} ForEachStart input slot {} out of bounds",
                node_idx,
                input.get()
            );
            assert!(
                item_slot.get() < slot_count,
                "node {} ForEachStart item_slot {} out of bounds",
                node_idx,
                item_slot.get()
            );
        }
        CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
            assert!(
                iterator_slot.get() < slot_count,
                "node {} ForEachNext iterator_slot {} out of bounds",
                node_idx,
                iterator_slot.get()
            );
        }
        CompiledNodeKind::ForEachJoin { output } => {
            assert!(
                output.get() < slot_count,
                "node {} ForEachJoin output slot {} out of bounds",
                node_idx,
                output.get()
            );
        }
        CompiledNodeKind::TogetherStart { .. } => {}
        CompiledNodeKind::TogetherBranch { accumulator, .. } => {
            assert!(
                accumulator.get() < slot_count,
                "node {} TogetherBranch accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::TogetherJoin { accumulator, .. } => {
            assert!(
                accumulator.get() < slot_count,
                "node {} TogetherJoin accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::CollectStart { source, .. } => {
            assert!(
                source.get() < slot_count,
                "node {} CollectStart source slot {} out of bounds",
                node_idx,
                source.get()
            );
        }
        CompiledNodeKind::CollectPage { collector_slot, .. }
        | CompiledNodeKind::CollectNext { collector_slot, .. }
        | CompiledNodeKind::CollectFinish { collector_slot } => {
            assert!(
                collector_slot.get() < slot_count,
                "node {} Collect collector_slot {} out of bounds",
                node_idx,
                collector_slot.get()
            );
        }
        CompiledNodeKind::ReduceStart {
            input, accumulator, ..
        } => {
            assert!(
                input.get() < slot_count,
                "node {} ReduceStart input slot {} out of bounds",
                node_idx,
                input.get()
            );
            assert!(
                accumulator.get() < slot_count,
                "node {} ReduceStart accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            ..
        } => {
            assert!(
                iterator_slot.get() < slot_count,
                "node {} ReduceNext iterator_slot {} out of bounds",
                node_idx,
                iterator_slot.get()
            );
            assert!(
                accumulator.get() < slot_count,
                "node {} ReduceNext accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::ReduceFinish { accumulator } => {
            assert!(
                accumulator.get() < slot_count,
                "node {} ReduceFinish accumulator slot {} out of bounds",
                node_idx,
                accumulator.get()
            );
        }
        CompiledNodeKind::RepeatStart { .. } => {}
        CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => {
            assert!(
                attempt_slot.get() < slot_count,
                "node {} RepeatAttempt attempt_slot {} out of bounds",
                node_idx,
                attempt_slot.get()
            );
        }
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
            assert!(
                attempt_slot.get() < slot_count,
                "node {} RepeatCheck attempt_slot {} out of bounds",
                node_idx,
                attempt_slot.get()
            );
        }
        CompiledNodeKind::RepeatFinish { result } => {
            assert!(
                result.get() < slot_count,
                "node {} RepeatFinish result slot {} out of bounds",
                node_idx,
                result.get()
            );
        }
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            assert!(
                deadline_slot.get() < slot_count,
                "node {} WaitUntil deadline_slot {} out of bounds",
                node_idx,
                deadline_slot.get()
            );
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            assert!(
                event.get() < slot_count,
                "node {} WaitEvent event slot {} out of bounds",
                node_idx,
                event.get()
            );
            if let Some(timeout) = timeout_slot {
                assert!(
                    timeout.get() < slot_count,
                    "node {} WaitEvent timeout_slot {} out of bounds",
                    node_idx,
                    timeout.get()
                );
            }
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            assert!(
                prompt.get() < slot_count,
                "node {} Ask prompt slot {} out of bounds",
                node_idx,
                prompt.get()
            );
            if let Some(timeout) = timeout_slot {
                assert!(
                    timeout.get() < slot_count,
                    "node {} Ask timeout_slot {} out of bounds",
                    node_idx,
                    timeout.get()
                );
            }
        }
        CompiledNodeKind::AskResume { answer } => {
            assert!(
                answer.get() < slot_count,
                "node {} AskResume answer slot {} out of bounds",
                node_idx,
                answer.get()
            );
        }
        CompiledNodeKind::RetryCheck { policy_slot, .. } => {
            assert!(
                policy_slot.get() < slot_count,
                "node {} RetryCheck policy_slot {} out of bounds",
                node_idx,
                policy_slot.get()
            );
        }
        CompiledNodeKind::ErrorHandler { error_slot, .. } => {
            if let Some(slot) = error_slot {
                assert!(
                    slot.get() < slot_count,
                    "node {} ErrorHandler error_slot {} out of bounds",
                    node_idx,
                    slot.get()
                );
            }
        }
        CompiledNodeKind::Finish { result } => {
            assert!(
                result.get() < slot_count,
                "node {} Finish result slot {} out of bounds",
                node_idx,
                result.get()
            );
        }
        _ => {}
    }
}

pub fn fuzz_generated_compare(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let parts_clone = parts.clone();
        let validated = vb_core::validate_compiled_workflow(&parts);
        let workflow = vb_core::CompiledWorkflow::try_from_parts(parts);
        assert!(
            validated.is_ok() == workflow.is_ok(),
            "validation and workflow construction must agree: validated={:?}, workflow={:?}",
            validated,
            workflow.is_ok()
        );
        if let (Ok(w1), Ok(w2)) = (
            workflow,
            vb_core::CompiledWorkflow::try_from_parts(parts_clone),
        ) {
            assert_eq!(
                w1.digest(),
                w2.digest(),
                "independent decode must yield same digest"
            );
            assert_eq!(
                w1.node_count(),
                w2.node_count(),
                "independent decode must yield same node count"
            );
            assert_eq!(
                w1.slot_count(),
                w2.slot_count(),
                "independent decode must yield same slot count"
            );
        }
    }
}

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

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(vb_core::WorkflowParts {
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
    }) else {
        return;
    };

    let budget_value = if data.len() >= 9 {
        let mut bytes = [0u8; 8];
        let src: [u8; 8] = match data.get(1..9) {
            Some(slice) => {
                let mut arr = [0u8; 8];
                let len = slice.len().min(8);
                let end = slice.len().min(len);
                if end > 0 {
                    arr[..end].copy_from_slice(&slice[..end]);
                }
                arr
            }
            None => [0u8; 8],
        };
        bytes.copy_from_slice(&src);
        u64::from_le_bytes(bytes)
    } else {
        u64::from(data.get(1).copied().unwrap_or(0))
    };
    let budget_value = budget_value.wrapping_rem(1000);

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

    let executed = run.executed();
    let executed_delta = executed.saturating_sub(initial_executed);

    if budget_value == 0 {
        assert!(
            executed_delta == 0,
            "zero budget should execute zero transitions, but executed {executed_delta}"
        );
        assert!(
            signal == vb_core::EngineSignal::StepBudgetExhausted,
            "zero budget should exhaust immediately, got {signal:?}"
        );
    }

    assert!(
        executed_delta <= budget_value,
        "executed {executed_delta} transitions with budget {budget_value}"
    );
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
        let node = build_fuzz_budget_node(i, kind_byte, node_count, slot_count);
        nodes.push(node);
    }

    let contract = vb_core::ResourceContract {
        max_slots: slot_count,
        ..vb_core::ResourceContract::DEFAULT
    };

    let entry = vb_core::StepIdx::ZERO;
    let result = vb_core::budget::WholeWorkflowBudget::compute(&nodes, entry, &contract);

    let Ok(budget) = result else {
        return;
    };

    if !nodes.is_empty() {
        assert!(
            budget.max_total_steps > 0,
            "max_total_steps must be positive for non-empty workflow"
        );
    }

    assert!(
        budget.max_total_slots >= slot_count as u64,
        "max_total_slots {} must be >= slot_count {}",
        budget.max_total_slots,
        slot_count
    );

    assert!(
        budget.max_fanout <= u16::MAX,
        "max_fanout {} exceeds u16::MAX",
        budget.max_fanout
    );

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

    let max_slot = slot_count.saturating_sub(1);
    let safe_slot = vb_core::SlotIdx::new(max_slot);

    let kind = match kind_byte.wrapping_rem(6) {
        0 => vb_core::CompiledNodeKind::Nop,
        1 => vb_core::CompiledNodeKind::Finish { result: safe_slot },
        2 => vb_core::CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        3 => vb_core::CompiledNodeKind::Copy { source: safe_slot },
        4 => {
            let body_idx = u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            let done_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::ForEachStart {
                input: safe_slot,
                item_slot: safe_slot,
                limit: 5,
                body: vb_core::StepIdx::new(body_idx),
                done: vb_core::StepIdx::new(done_idx),
            }
        }
        _ => {
            let branch_idx =
                u16::try_from(index.saturating_add(1).min(node_count.saturating_sub(1)))
                    .unwrap_or(0);
            let join_idx = u16::try_from(index.saturating_add(2).min(node_count.saturating_sub(1)))
                .unwrap_or(0);
            vb_core::CompiledNodeKind::TogetherStart {
                branches: vec![
                    vb_core::StepIdx::new(branch_idx),
                    vb_core::StepIdx::new(branch_idx),
                ]
                .into_boxed_slice(),
                join: vb_core::StepIdx::new(join_idx),
            }
        }
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

pub fn fuzz_accessor_traversal(data: &[u8]) {
    if data.len() < 4 {
        return;
    }

    let Some(&byte0) = data.first() else {
        return;
    };
    let Some(&byte1) = data.get(1) else {
        return;
    };
    let slot_count = u16::from(byte0.wrapping_rem(16)).saturating_add(1);
    let accessor_count = usize::from(byte1.wrapping_rem(8)).saturating_add(1);

    let mut accessors: Vec<vb_core::AccessorProgram> = Vec::new();
    let mut offset = 2usize;
    for _ in 0..accessor_count {
        let root_byte = data.get(offset).copied().unwrap_or(0);
        let safe_slot_count: u16 = match slot_count {
            0 => 1u16,
            n => n,
        };
        #[allow(clippy::arithmetic_side_effects)]
        let root = vb_core::SlotIdx::new(u16::from(root_byte).wrapping_rem(safe_slot_count));
        offset = offset.saturating_add(1);

        let path_len_byte = data.get(offset).copied().unwrap_or(0);
        let path_len = usize::from(path_len_byte.wrapping_rem(4));
        offset = offset.saturating_add(1);

        let mut path: Vec<vb_core::PathSegment> = Vec::new();
        for _ in 0..path_len {
            if offset >= data.len() {
                break;
            }
            let seg_byte = data.get(offset).copied().unwrap_or(0);
            offset = offset.saturating_add(1);
            let segment = if seg_byte.is_multiple_of(2) {
                vb_core::PathSegment::Field(vb_core::SymbolId::new(
                    u32::from(seg_byte).wrapping_rem(16),
                ))
            } else {
                vb_core::PathSegment::Index(u32::from(seg_byte).wrapping_rem(8))
            };
            path.push(segment);
            assert!(
                path.len() <= FUZZ_MAX_ACCESSOR_DEPTH,
                "accessor path depth {} exceeds max {}",
                path.len(),
                FUZZ_MAX_ACCESSOR_DEPTH
            );
            if path.len() >= FUZZ_MAX_ACCESSOR_DEPTH {
                break;
            }
        }

        accessors.push(vb_core::AccessorProgram {
            root,
            path: path.into_boxed_slice(),
        });

        if offset >= data.len() {
            break;
        }
    }

    let max_slot = slot_count.saturating_sub(1);
    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_accessor"),
        digest: vb_core::WorkflowDigest::from_bytes([0xE0; 32]),
        nodes: vec![vb_core::CompiledNode {
            id: vb_core::StepIdx::new(0),
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::new(max_slot),
            },
        }]
        .into(),
        expressions: Box::new([]),
        accessors: accessors.into(),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };

    let mut store = vb_core::ValueStore::new();

    let Ok(sym_a) = store.insert_symbol(Box::<str>::from("field_a")) else {
        return;
    };
    let _ = sym_a;
    let Ok(list_id) = store.insert_list(
        vec![
            vb_core::SlotValue::I64(10),
            vb_core::SlotValue::I64(20),
            vb_core::SlotValue::I64(30),
        ]
        .into_boxed_slice(),
    ) else {
        return;
    };
    let Ok(obj_id) = store.insert_object(
        vec![
            vb_core::value_store::ObjectField {
                key: vb_core::SymbolId::new(0),
                value: vb_core::SlotValue::Bool(true),
                taint: vb_core::value::Taint::Clean,
            },
            vb_core::value_store::ObjectField {
                key: vb_core::SymbolId::new(1),
                value: vb_core::SlotValue::I64(42),
                taint: vb_core::value::Taint::Clean,
            },
        ]
        .into_boxed_slice(),
    ) else {
        return;
    };

    let mut run_with_data = match vb_core::RunFrame::new(
        vb_core::RunId::new(4),
        vb_core::StepIdx::ZERO,
        workflow.node_count(),
        slot_count,
    ) {
        Ok(r) => r,
        Err(_) => return,
    };

    if max_slot > 0 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(0),
                vb_core::SlotValue::Null,
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 1 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(1),
                vb_core::SlotValue::Bool(true),
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 2 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(2),
                vb_core::SlotValue::I64(7),
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 3 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(3),
                vb_core::SlotValue::List(list_id),
                vb_core::Taint::Clean,
            )
            .ok();
    }
    if slot_count > 4 {
        run_with_data
            .write_slot_with_taint(
                vb_core::SlotIdx::new(4),
                vb_core::SlotValue::Object(obj_id),
                vb_core::Taint::Clean,
            )
            .ok();
    }

    let mut i: u16 = 0;
    loop {
        let accessor_idx = vb_core::AccessorIdx::new(i);
        if workflow.accessor(accessor_idx).is_none() {
            break;
        }
        drop(vb_core::engine::eval_accessor_with_store(
            &workflow,
            &run_with_data,
            &mut store,
            accessor_idx,
        ));
        i = i.saturating_add(1);
        if i == 0 {
            break;
        }
    }
}

pub fn fuzz_slot_value_roundtrip(data: &[u8]) {
    let Ok(decoded): Result<vb_core::SlotValue, _> = postcard::from_bytes(data) else {
        return;
    };

    let Ok(re_encoded): Result<Vec<u8>, _> = postcard::to_allocvec(&decoded) else {
        return;
    };

    if data.len() == re_encoded.len() {
        let mut matching = true;
        for i in 0..data.len() {
            if data.get(i) != re_encoded.get(i) {
                matching = false;
                break;
            }
        }
        if matching {
            let Ok(_re_decoded): Result<vb_core::SlotValue, _> = postcard::from_bytes(&re_encoded)
            else {
                return;
            };
        }
    }

    let store = vb_core::ValueStore::new();
    let display = decoded.display_with_store(&store);
    assert!(
        !display.is_empty(),
        "display_with_store must produce non-empty output"
    );

    let type_name = decoded.type_name();
    assert!(!type_name.is_empty(), "type_name must be non-empty");

    let truthy = decoded.is_true();
    assert_eq!(truthy, decoded.is_true(), "is_true must be deterministic");
}

pub fn fuzz_collect_page_pagination(data: &[u8]) {
    if data.is_empty() {
        return;
    }

    let slot_count = u16::from(data[0].wrapping_rem(16)).saturating_add(1);
    let list_len = usize::from(data[0].wrapping_rem(8));
    let page_size =
        usize::from(data.get(1).copied().unwrap_or(1).wrapping_rem(8)).saturating_add(1);

    let Ok(mut run) = vb_core::RunFrame::new(
        vb_core::RunId::new(1),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };

    let mut store = vb_core::ValueStore::new();

    let items: Vec<vb_core::SlotValue> = (0..list_len)
        .map(|i| vb_core::SlotValue::I64(i64::try_from(i).unwrap_or(0)))
        .collect();

    let list_id = match store.insert_list(items.into_boxed_slice()) {
        Ok(id) => id,
        Err(_) => return,
    };

    let _ = run.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::List(list_id),
        vb_core::Taint::Clean,
    );

    use vb_runtime::primitives::collect::{CollectStates, collect_page, collect_start};

    let mut states = CollectStates::new();
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        vb_core::SlotIdx::new(0),
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
    );
    match result {
        Ok(status) => {
            assert!(
                matches!(status, vb_core::EngineSignal::Continue),
                "collect_page on list slot must return Continue, got {status:?}"
            );
            if list_len == 0 {}
        }
        Err(_error) => {}
    }

    let Ok(mut run_zero) = vb_core::RunFrame::new(
        vb_core::RunId::new(3),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };
    let _ = run_zero.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::List(list_id),
        vb_core::Taint::Clean,
    );
    let mut states_zero = CollectStates::new();
    let zero_page_result = collect_start(
        &mut run_zero,
        &mut store,
        &mut states_zero,
        vb_core::SlotIdx::new(0),
        page_size as u32,
        page_size as u32,
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
        None,
        None,
    );
    if page_size == 0 {
        assert!(
            zero_page_result.is_err(),
            "collect_start with page_size=0 must return error"
        );
    }
    if page_size > 0
        && list_len > 0
        && list_len < page_size
    {
        match zero_page_result {
            Ok(signal) => {
                assert!(
                    matches!(signal,
                        vb_core::EngineSignal::Continue
                        | vb_core::EngineSignal::Finished(..)
                    ),
                    "collect_start single-page signal unexpected: {signal:?}"
                );
            }
            Err(_) => {}
        }
    }

    let Ok(mut run_non_list) = vb_core::RunFrame::new(
        vb_core::RunId::new(2),
        vb_core::StepIdx::ZERO,
        2,
        slot_count,
    ) else {
        return;
    };

    let _ = run_non_list.write_slot_with_taint(
        vb_core::SlotIdx::new(0),
        vb_core::SlotValue::I64(42),
        vb_core::Taint::Clean,
    );

    let mut states2 = CollectStates::new();
    let non_list_result = collect_page(
        &mut run_non_list,
        &mut store,
        &mut states2,
        vb_core::SlotIdx::new(0),
        vb_core::StepIdx::new(1),
        vb_core::StepIdx::new(1),
    );
    assert!(
        non_list_result.is_err(),
        "collect_page on non-list slot must return error"
    );
}

pub fn fuzz_step_budget_new(data: &[u8]) {
    if data.is_empty() {
        return;
    }

    let budget_value = if data.len() >= 8 {
        let mut bytes = [0u8; 8];
        let src = &data[..8.min(data.len())];
        bytes[..src.len()].copy_from_slice(src);
        u64::from_le_bytes(bytes)
    } else {
        u64::from(data[0])
    };

    let budget = vb_core::StepBudget::new(budget_value);
    let remaining = budget.remaining();

    assert!(
        remaining <= vb_core::limits::MAX_STEP_BUDGET,
        "StepBudget::new({}) produced remaining={}, exceeds MAX_STEP_BUDGET={}",
        budget_value,
        remaining,
        vb_core::limits::MAX_STEP_BUDGET
    );

    let expected = budget_value.min(vb_core::limits::MAX_STEP_BUDGET);
    assert!(
        remaining == expected,
        "StepBudget::new({}) remaining={}, expected {}",
        budget_value,
        remaining,
        expected
    );

    let mut mutable_budget = budget;
    let result = mutable_budget.try_take();
    assert!(result.is_ok(), "try_take must not error");

    if expected > 0 {
        let ok = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        let decremented = match expected.checked_sub(1) {
            Some(value) => value,
            None => return,
        };
        assert!(ok, "try_take should succeed when budget > 0");
        assert_eq!(
            mutable_budget.remaining(),
            decremented,
            "remaining should decrement by 1 after successful try_take"
        );
    } else {
        let ok = match result {
            Ok(value) => value,
            Err(_) => return,
        };
        assert!(!ok, "try_take should return false when budget is 0");
        assert_eq!(
            mutable_budget.remaining(),
            0,
            "remaining should stay 0 after failed try_take"
        );
    }
}
