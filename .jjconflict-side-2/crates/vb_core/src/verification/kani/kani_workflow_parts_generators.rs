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
    input_schema::kind()
}

fn arbitrary_input_constraint() -> CompiledInputConstraint {
    input_schema::constraint(2)
}

fn bounded_workflow_entry(node_count: u8) -> StepIdx {
    let entry: u8 = kani::any();
    StepIdx::new(u16::from(
        entry.checked_rem(node_count).map_or(0, |value| value),
    ))
}

include!("kani_workflow_symbolic_collections.rs");
