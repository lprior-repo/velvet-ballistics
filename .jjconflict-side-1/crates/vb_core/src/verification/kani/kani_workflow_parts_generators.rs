fn bounded_workflow_nodes() -> (u8, Box<[CompiledNode]>) {
    let count: u8 = kani::any();
    kani::assume(count <= 8);
    (count, bounded_symbolic_items(count))
}

fn bounded_workflow_expressions() -> Box<[ExprProgram]> {
    let count: u8 = kani::any();
    kani::assume(count <= 4);
    (0..count).map(|_| kani::any()).collect()
}

fn bounded_workflow_constants() -> Box<[ConstValue]> {
    let count: u8 = kani::any();
    kani::assume(count <= 4);
    (0..count).map(|_| kani::any()).collect()
}

fn bounded_workflow_step_names() -> Box<[Box<str>]> {
    let count: u8 = kani::any();
    kani::assume(count <= 4);
    (0..count).map(kani_step_name).collect()
}

fn bounded_workflow_input_slots() -> Box<[CompiledInputSlot]> {
    let count: u8 = kani::any();
    kani::assume(count <= 4);
    (0..count).map(arbitrary_input_slot).collect()
}

fn arbitrary_input_slot(index: u8) -> CompiledInputSlot {
    CompiledInputSlot {
        slot: SlotIdx::new(u16::from(index)),
        kind: arbitrary_input_kind(),
        constraint: arbitrary_input_constraint(),
    }
}

fn arbitrary_input_kind() -> InputSlotKind {
    match kani::any::<u8>() {
        0 => InputSlotKind::Null,
        1 => InputSlotKind::Bool,
        2 => InputSlotKind::I64,
        3 => InputSlotKind::F64,
        4 => InputSlotKind::Symbol,
        5 => InputSlotKind::List,
        6 => InputSlotKind::Object,
        7 => InputSlotKind::Blob,
        _ => InputSlotKind::Any,
    }
}

fn arbitrary_input_constraint() -> CompiledInputConstraint {
    let (min, max) = ordered_optional_bounds();
    let (min_length, max_length) = ordered_optional_length_bounds();
    CompiledInputConstraint {
        presence: Presence::from_bool(kani::any::<bool>()),
        nullability: Nullability::from_bool(kani::any::<bool>()),
        default_value: None,
        min,
        max,
        min_length,
        max_length,
        element: None,
        extra: ExtraFields::Unspecified,
        secretness: Secretness::from_bool(kani::any::<bool>()),
    }
}

fn bounded_workflow_entry(node_count: u8) -> StepIdx {
    let entry: u8 = kani::any();
    StepIdx::new(u16::from(
        entry.checked_rem(node_count).map_or(0, |value| value),
    ))
}

include!("kani_workflow_symbolic_collections.rs");
