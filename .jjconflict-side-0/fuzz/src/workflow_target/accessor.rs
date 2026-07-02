//! Accessor traversal fuzz target body.

const FUZZ_MAX_ACCESSOR_DEPTH: usize = 16;

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
    let accessors = build_accessors(data, slot_count, accessor_count);
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
    let Ok(obj_id) = object_fixture(&mut store) else {
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
    seed_accessor_slots(&mut run_with_data, slot_count, max_slot, list_id, obj_id);
    evaluate_accessors(&workflow, &run_with_data, &mut store);
}

fn build_accessors(
    data: &[u8],
    slot_count: u16,
    accessor_count: usize,
) -> Vec<vb_core::AccessorProgram> {
    let mut accessors = Vec::new();
    let mut offset = 2usize;
    for _ in 0..accessor_count {
        let root_byte = data.get(offset).copied().unwrap_or(0);
        let root = vb_core::SlotIdx::new(u16::from(root_byte).wrapping_rem(slot_count.max(1)));
        offset = offset.saturating_add(1);
        let path_len = usize::from(data.get(offset).copied().unwrap_or(0).wrapping_rem(4));
        offset = offset.saturating_add(1);
        let mut path = Vec::new();
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
            assert!(path.len() <= FUZZ_MAX_ACCESSOR_DEPTH);
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
    accessors
}

fn object_fixture(store: &mut vb_core::ValueStore) -> Result<vb_core::ObjectId, vb_core::CoreError> {
    store.insert_object(
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
    )
}

fn seed_accessor_slots(
    run: &mut vb_core::RunFrame,
    slot_count: u16,
    max_slot: u16,
    list_id: vb_core::ListId,
    obj_id: vb_core::ObjectId,
) {
    if max_slot > 0 {
        run.write_slot_with_taint(vb_core::SlotIdx::new(0), vb_core::SlotValue::Null, vb_core::Taint::Clean).ok();
    }
    if slot_count > 1 {
        run.write_slot_with_taint(vb_core::SlotIdx::new(1), vb_core::SlotValue::Bool(true), vb_core::Taint::Clean).ok();
    }
    if slot_count > 2 {
        run.write_slot_with_taint(vb_core::SlotIdx::new(2), vb_core::SlotValue::I64(7), vb_core::Taint::Clean).ok();
    }
    if slot_count > 3 {
        run.write_slot_with_taint(vb_core::SlotIdx::new(3), vb_core::SlotValue::List(list_id), vb_core::Taint::Clean).ok();
    }
    if slot_count > 4 {
        run.write_slot_with_taint(vb_core::SlotIdx::new(4), vb_core::SlotValue::Object(obj_id), vb_core::Taint::Clean).ok();
    }
}

fn evaluate_accessors(
    workflow: &vb_core::CompiledWorkflow,
    run: &vb_core::RunFrame,
    store: &mut vb_core::ValueStore,
) {
    let mut i: u16 = 0;
    loop {
        let accessor_idx = vb_core::AccessorIdx::new(i);
        if workflow.accessor(accessor_idx).is_none() {
            break;
        }
        drop(vb_core::engine::eval_accessor_with_store(
            workflow,
            run,
            store,
            accessor_idx,
        ));
        i = i.saturating_add(1);
        if i == 0 {
            break;
        }
    }
}
