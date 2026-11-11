use vb_compile::ast::{AstExpression, AstValue, StepKindAst, WorkflowAst};
use vb_compile::expression::{ExpressionHelper, ExpressionLiteral, ParsedExpression};
use vb_compile::{CompileErrors, YamlCompiler, compile_workflow};
use vb_core::engine::eval_expr_with_store;
use vb_core::{
    CompiledWorkflow, ExprIdx, ExprOp, InputSlotKind, ObjectField, RunFrame, RunId, SlotIdx,
    SlotValue, StepIdx, Taint, ValueStore,
};

pub(super) struct HasArtifacts {
    pub(super) compiled: CompiledWorkflow,
    pub(super) ast: WorkflowAst,
}

pub(super) fn input_section(inputs: &[(&str, &str)]) -> String {
    if inputs.is_empty() {
        return "inputs: {}".to_string();
    }
    let declarations = inputs
        .iter()
        .map(|(name, schema)| format!("  {name}: {schema}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("inputs:\n{declarations}")
}

pub(super) fn only_error(
    errors: &vb_compile::CompileErrors,
) -> Result<&vb_compile::CompileError, String> {
    let count = errors.0.len();
    if count != 1 {
        return Err(format!("expected exactly one compile error, got {count}"));
    }
    errors
        .0
        .first()
        .ok_or_else(|| "expected exactly one compile error".to_string())
}

fn source_with_set(inputs: &[(&str, &str)], expression: &str) -> String {
    let declarations = input_section(inputs);
    format!(
        "version: velvet-ballistics/v1\nname: has_test\nwhen:\n  manual: {{}}\n{declarations}\nsteps:\n  - id: check\n    set: {{ output: result, value: '{expression}' }}\n  - id: done\n    finish: {{ result: result }}\n"
    )
}

fn source_with_choose(inputs: &[(&str, &str)], expression: &str) -> String {
    let declarations = input_section(inputs);
    format!(
        "version: velvet-ballistics/v1\nname: has_test\nwhen:\n  manual: {{}}\n{declarations}\nsteps:\n  - id: route\n    choose:\n      condition: '{expression}'\n      on_true: 1\n      on_false: 1\n  - id: done\n    finish: {{ result: null }}\n"
    )
}

pub(super) fn workflow_compile(expression: &str) -> Result<CompiledWorkflow, CompileErrors> {
    compile_workflow(source_with_set(&[], expression).as_bytes())
}

pub(super) fn compile_with_inputs(
    inputs: &[(&str, &str)],
    expression: &str,
) -> Result<CompiledWorkflow, CompileErrors> {
    compile_workflow(source_with_set(inputs, expression).as_bytes())
}

pub(super) fn compile_artifacts_with_inputs(
    inputs: &[(&str, &str)],
    expression: &str,
) -> Result<HasArtifacts, CompileErrors> {
    let source = source_with_set(inputs, expression);
    let compiled = compile_workflow(source.as_bytes())?;
    let ast_source = source_with_choose(inputs, expression);
    let ast = YamlCompiler::default().parse_ast(ast_source.as_bytes())?;
    Ok(HasArtifacts { compiled, ast })
}

pub(super) fn parse_ast_with_inputs(
    inputs: &[(&str, &str)],
    expression: &str,
) -> Result<WorkflowAst, CompileErrors> {
    YamlCompiler::default().parse_ast(source_with_choose(inputs, expression).as_bytes())
}

pub(super) fn assert_compiled_has(
    artifacts: &HasArtifacts,
    schemas: &[(&str, &str, InputSlotKind)],
) -> Result<(), String> {
    assert_input_schema(&artifacts.ast, schemas)?;
    assert_slot_kinds(&artifacts.compiled, schemas)?;
    assert_eq!(
        expression_ops(&artifacts.compiled)?,
        &[
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::LoadSlot(SlotIdx::new(2)),
            ExprOp::Has
        ]
    );
    assert_executable_has(artifacts)
}

pub(super) fn assert_single_any_slot_has(artifacts: &HasArtifacts) -> Result<(), String> {
    assert_input_schema(&artifacts.ast, &[("value", "any", InputSlotKind::Any)])?;
    assert_eq!(artifacts.compiled.input_slots().len(), 1);
    let Some(slot) = artifacts.compiled.input_slots().first() else {
        return Err("compiled Any input slot missing".to_string());
    };
    assert_eq!(slot.slot, SlotIdx::new(1));
    assert_eq!(slot.kind, InputSlotKind::Any);
    assert_eq!(
        expression_ops(&artifacts.compiled)?,
        &[
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::LoadSlot(SlotIdx::new(1)),
            ExprOp::Has
        ]
    );
    Ok(())
}

pub(super) fn assert_input_schema(
    ast: &WorkflowAst,
    schemas: &[(&str, &str, InputSlotKind)],
) -> Result<(), String> {
    assert_eq!(ast.inputs.len(), schemas.len());
    for (input, (name, schema, _)) in ast.inputs.iter().zip(schemas) {
        assert_eq!(input.name.as_ref(), *name);
        assert_eq!(input.value, AstValue::Text((*schema).into()));
    }
    Ok(())
}

fn assert_slot_kinds(
    compiled: &CompiledWorkflow,
    schemas: &[(&str, &str, InputSlotKind)],
) -> Result<(), String> {
    assert_eq!(compiled.input_slots().len(), schemas.len());
    for (index, (slot, (_, _, kind))) in compiled.input_slots().iter().zip(schemas).enumerate() {
        let input_offset = index
            .checked_add(1)
            .ok_or_else(|| "input slot offset overflow".to_string())?;
        let expected = u16::try_from(input_offset).map_err(|error| error.to_string())?;
        assert_eq!(slot.slot, SlotIdx::new(expected));
        assert_eq!(slot.kind, *kind);
    }
    Ok(())
}

fn expression_ops(compiled: &CompiledWorkflow) -> Result<&[ExprOp], String> {
    compiled
        .expression(expression_index(compiled)?)
        .map(|program| program.ops.as_ref())
        .ok_or_else(|| "compiled expression missing".to_string())
}

fn expression_index(compiled: &CompiledWorkflow) -> Result<ExprIdx, String> {
    let node = compiled
        .node(StepIdx::new(0))
        .ok_or_else(|| "compiled set node missing".to_string())?;
    match &node.kind {
        vb_core::CompiledNodeKind::EvalExpr { expr } => Ok(*expr),
        vb_core::CompiledNodeKind::Choose { branches, .. } => branches
            .first()
            .map(|branch| branch.condition)
            .ok_or_else(|| "compiled choose branch missing".to_string()),
        other => Err(format!("compiled step lacked expression branch: {other:?}")),
    }
}

fn assert_executable_has(artifacts: &HasArtifacts) -> Result<(), String> {
    let mut store = ValueStore::new();
    let (object, key) = runtime_has_values(&mut store)?;
    let frame = runtime_has_frame(artifacts, object, key)?;
    let (value, taint) = eval_expr_with_store(
        &artifacts.compiled,
        &frame,
        &mut store,
        expression_index(&artifacts.compiled)?,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(value, SlotValue::Bool(true));
    assert_eq!(taint, Taint::Clean);
    Ok(())
}

fn runtime_has_values(store: &mut ValueStore) -> Result<(SlotValue, SlotValue), String> {
    let key = store
        .insert_symbol("key")
        .map_err(|error| error.to_string())?;
    let object = store
        .insert_object(vec![ObjectField::clean(key, SlotValue::Bool(true))])
        .map_err(|error| error.to_string())?;
    Ok((SlotValue::Object(object), SlotValue::Symbol(key)))
}

fn runtime_has_frame(
    artifacts: &HasArtifacts,
    object: SlotValue,
    key: SlotValue,
) -> Result<RunFrame, String> {
    let mut frame = RunFrame::new(
        RunId::new(9),
        StepIdx::new(0),
        2,
        artifacts.compiled.slot_count(),
    )
    .map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(SlotIdx::new(1), object, Taint::Clean)
        .map_err(|error| error.to_string())?;
    frame
        .write_slot_with_taint(SlotIdx::new(2), key, Taint::Clean)
        .map_err(|error| error.to_string())?;
    Ok(frame)
}

pub(super) fn assert_ast_has(
    ast: &WorkflowAst,
    schemas: &[(&str, &str, InputSlotKind)],
    references: &[&str],
    trailing_i64: Option<i64>,
) -> Result<(), String> {
    assert_input_schema(ast, schemas)?;
    let step = ast
        .steps
        .first()
        .ok_or_else(|| "choose step missing".to_string())?;
    let StepKindAst::Choose { condition, .. } = &step.kind else {
        return Err("first AST step was not choose".to_string());
    };
    let AstExpression::Parsed(parsed) = condition else {
        return Err(format!("choose condition was not parsed: {condition:?}"));
    };
    assert_ast_has_args(parsed, references, trailing_i64)
}

fn assert_ast_has_args(
    parsed: &ParsedExpression,
    references: &[&str],
    trailing_i64: Option<i64>,
) -> Result<(), String> {
    let ParsedExpression::HelperCall { name, args } = parsed else {
        return Err(format!("condition was not a helper call: {parsed:?}"));
    };
    assert_eq!(*name, ExpressionHelper::Has);
    assert_eq!(
        args.len(),
        expected_ast_arg_count(references.len(), trailing_i64)?
    );
    assert_ast_references(args, references)?;
    assert_ast_trailing_i64(args, trailing_i64)
}

fn expected_ast_arg_count(references: usize, trailing_i64: Option<i64>) -> Result<usize, String> {
    if trailing_i64.is_some() {
        references
            .checked_add(1)
            .ok_or_else(|| "AST argument count overflow".to_string())
    } else {
        Ok(references)
    }
}

fn assert_ast_references(args: &[ParsedExpression], references: &[&str]) -> Result<(), String> {
    for (actual, expected) in args.iter().zip(references) {
        assert_eq!(actual, &ParsedExpression::Reference((*expected).into()));
    }
    Ok(())
}

fn assert_ast_trailing_i64(
    args: &[ParsedExpression],
    trailing_i64: Option<i64>,
) -> Result<(), String> {
    if let Some(value) = trailing_i64 {
        let Some(actual) = args.last() else {
            return Err("expected trailing integer AST argument".to_string());
        };
        assert_eq!(
            actual,
            &ParsedExpression::Literal(ExpressionLiteral::I64(value))
        );
    }
    Ok(())
}
