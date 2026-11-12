use vb_compile::{CompileErrors, compile_workflow};
use vb_core::engine::eval_expr_with_store;
use vb_core::value_store::{ObjectField, ValueStoreCheckpointV1, compose_compiler_symbols};
use vb_core::workflow::WorkflowIrV2;
use vb_core::{
    CompiledWorkflow, ExprIdx, ObjectId, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, SymbolId,
    Taint, ValueStore, WorkflowDigest,
};

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn compile_expression(expression: &str) -> Result<CompiledWorkflow, CompileErrors> {
    compile_workflow(format!(
        "version: velvet-ballistics/v1\nname: bound_expression\nwhen:\n  manual: {{}}\ninputs:\n  obj: any\nsteps:\n  - id: check\n    set: {{ output: result, value: '{expression}' }}\n  - id: done\n    finish: {{ result: result }}\n"
    ).as_bytes())
}

pub(super) fn compile_variable(text: &str) -> Result<CompiledWorkflow, CompileErrors> {
    compile_workflow(format!(
        "version: velvet-ballistics/v1\nname: bound_variable\nwhen:\n  manual: {{}}\nvars:\n  word: '{text}'\nsteps:\n  - id: done\n    finish: {{ result: '$vars.word' }}\n"
    ).as_bytes())
}

pub(super) fn independent_v2_digest(
    mut ir: WorkflowIrV2,
) -> Result<WorkflowDigest, postcard::Error> {
    ir.parts.digest = WorkflowDigest::from_bytes([0; 32]);
    let mut preimage = vec![0x01, 0xff, b'V', b'B'];
    preimage.extend(postcard::to_allocvec(&ir)?);
    Ok(WorkflowDigest::from_bytes(blake3::hash(&preimage).into()))
}

fn owned_input() -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: vec!["unused".into(), "key".into(), "input-only".into()],
        lists: vec![],
        list_taints: vec![],
        objects: vec![Box::from([ObjectField::clean(
            SymbolId::new(1),
            SlotValue::I64(7),
        )])],
        blobs: vec![],
        max_arena_entries: 8,
    }
}

pub(super) fn execute_has(
    key: &str,
    expected_symbols: &[Box<str>],
    expected_key: SymbolId,
    expected: bool,
) -> TestResult {
    let workflow = compile_expression(&format!("has($input.obj, \"{key}\")"))?;
    let mut store = prepared_store(&workflow, expected_symbols, expected_key)?;
    let frame = input_frame(&workflow)?;
    assert_eq!(
        eval_expr_with_store(&workflow, &frame, &mut store, ExprIdx::new(0))?,
        (SlotValue::Bool(expected), Taint::Clean)
    );
    Ok(())
}

fn prepared_store(
    workflow: &CompiledWorkflow,
    expected_symbols: &[Box<str>],
    expected_key: SymbolId,
) -> Result<ValueStore, Box<dyn std::error::Error>> {
    let prefix = workflow
        .symbol_payloads()
        .ok_or("missing compiler prefix")?;
    let input = owned_input();
    let (store, roots) =
        compose_compiler_symbols(prefix, &input, &[SlotValue::Object(ObjectId::new(0))])?;
    assert_eq!(roots.as_ref(), &[SlotValue::Object(ObjectId::new(0))]);
    assert_eq!(store.checkpoint_v1().symbols.as_slice(), expected_symbols);
    assert_eq!(
        store.object_field_with_taint(ObjectId::new(0), expected_key),
        Ok((SlotValue::I64(7), Taint::Clean))
    );
    Ok(store)
}

fn input_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, Box<dyn std::error::Error>> {
    let mut frame = RunFrame::new(RunId::new(9), StepIdx::new(0), 2, workflow.slot_count())?;
    frame.write_slot_with_taint(
        SlotIdx::new(1),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )?;
    Ok(frame)
}

pub(super) fn declarations_source() -> &'static str {
    "version: velvet-ballistics/v1\nname: declarations\nwhen:\n  manual: {}\nvars:\n  first: z\n  second: a\nsteps:\n  - id: left\n    set: { output: one, value: '\"a\"' }\n  - id: right\n    set: { output: two, value: '\"z\"' }\n  - id: done\n    finish: { result: two }\n"
}

pub(super) fn nested_source() -> &'static str {
    "version: velvet-ballistics/v1\nname: nested_literals\nwhen:\n  manual: {}\ninputs:\n  items: {is: list, of: any, max: 2}\nvars:\n  prefix: decl\nsteps:\n  - id: seed\n    set: {output: seed_value, value: '\"outer\"'}\n  - id: each\n    for_each:\n      input: '$input.items'\n      variable: entry\n      steps:\n        - id: left\n          set: {output: nested_left, value: '\"inner\"'}\n        - id: right\n          set: {output: nested_right, value: '\"outer\"'}\n  - id: tail\n    set: {output: tail_value, value: '\"last\"'}\n  - id: done\n    finish: {result: tail_value}\n"
}
