use super::*;
use crate::source_symbol_binding::intern_source_text;
use std::collections::HashMap;
use vb_core::{ConstIdx, ExprOp, ExprProgram, SymbolId};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn compile_literals(builder: &mut SlotCompiler, text: &str) -> Result<(), CompileErrors> {
    let outputs = HashMap::new();
    let types = HashMap::new();
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: &outputs,
        slot_types: &types,
        parent: None,
    };
    compile_canonical_expression(text, scope, builder).map(|_| ())
}

fn equality_program(left: u16, right: u16) -> ExprProgram {
    ExprProgram {
        ops: Box::from([
            ExprOp::LoadConst(ConstIdx::new(left)),
            ExprOp::LoadConst(ConstIdx::new(right)),
            ExprOp::Eq,
        ]),
        max_stack: 2,
    }
}

fn expected_has_program() -> ExprProgram {
    ExprProgram {
        ops: Box::from([
            ExprOp::LoadSlot(SlotIdx::new(4)),
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::Has,
        ]),
        max_stack: 2,
    }
}

fn expected_capture_program() -> ExprProgram {
    ExprProgram {
        ops: Box::from([
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadSlot(SlotIdx::new(5)),
            ExprOp::Eq,
        ]),
        max_stack: 2,
    }
}

fn expected_nested_program() -> ExprProgram {
    ExprProgram {
        ops: Box::from([
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Eq,
            ExprOp::LoadConst(ConstIdx::new(2)),
            ExprOp::LoadConst(ConstIdx::new(3)),
            ExprOp::Eq,
            ExprOp::And,
        ]),
        max_stack: 3,
    }
}

#[test]
fn nested_expressions_preserve_depth_first_written_order() -> TestResult {
    let mut builder = SlotCompiler::new();
    compile_literals(&mut builder, "(\"z\" == \"a\") and (\"b\" == \"z\")")?;
    assert_eq!(
        builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(2)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    assert_eq!(builder.expressions, vec![expected_nested_program()]);
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("z"), Box::from("a"), Box::from("b"),])
    );
    Ok(())
}

#[test]
fn leading_and_trailing_spaces_are_distinct_payloads() -> TestResult {
    let mut builder = SlotCompiler::new();
    compile_literals(&mut builder, "\"key\" == \" key\"")?;
    compile_literals(&mut builder, "\"key \" == \"key\"")?;
    assert_eq!(
        builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(2)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([
            Box::<str>::from("key"),
            Box::from(" key"),
            Box::from("key "),
        ])
    );
    Ok(())
}

#[test]
fn separate_canonical_expressions_share_first_definition_ids() -> TestResult {
    let mut builder = SlotCompiler::new();
    compile_literals(&mut builder, "\"z\" == \"a\"")?;
    compile_literals(&mut builder, "\"a\" == \"z\"")?;
    assert_eq!(
        builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    assert_eq!(
        builder.expressions,
        vec![equality_program(0, 1), equality_program(2, 3)]
    );
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("z"), Box::from("a")])
    );
    Ok(())
}

#[test]
fn helper_receiver_still_precedes_the_interned_key() -> TestResult {
    let outputs = HashMap::from([("obj".into(), SlotIdx::new(4))]);
    let steps = HashMap::new();
    let types = HashMap::from([(SlotIdx::new(4), ExprType::Object)]);
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: &steps,
        slot_types: &types,
        parent: None,
    };
    let mut builder = SlotCompiler::new();
    let (_, ty) = compile_canonical_expression("has($obj, \"key\")", scope, &mut builder)?;
    assert_eq!(ty, ExprType::Bool);
    assert_eq!(
        builder.constants,
        vec![ConstValue::Symbol(SymbolId::new(0))]
    );
    assert_eq!(builder.expressions, vec![expected_has_program()]);
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("key")])
    );
    Ok(())
}

#[test]
fn local_alias_capture_is_delegated_without_symbol_side_effects() -> TestResult {
    let outputs = HashMap::from([("item".into(), SlotIdx::new(2))]);
    let steps = HashMap::new();
    let types = HashMap::from([(SlotIdx::new(5), ExprType::Text)]);
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: &steps,
        slot_types: &types,
        parent: None,
    };
    let mut builder = SlotCompiler::new();
    compile_canonical_expression_with_binding(
        "\"key\" == $item",
        scope,
        Some(("item", SlotIdx::new(5))),
        &mut builder,
    )?;
    assert_eq!(builder.expressions, vec![expected_capture_program()]);
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("key")])
    );
    Ok(())
}

#[test]
fn helper_arguments_intern_in_written_order_and_reuse_bytes() -> TestResult {
    let mut builder = SlotCompiler::new();
    compile_literals(&mut builder, "starts_with(\"z\", \"a\")")?;
    compile_literals(&mut builder, "starts_with(\"a\", \"z\")")?;
    assert_eq!(
        builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(0)),
        ]
    );
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("z"), Box::from("a")])
    );
    Ok(())
}

#[test]
fn empty_and_multibyte_payloads_preserve_exact_bytes() -> TestResult {
    let mut builder = SlotCompiler::new();
    compile_literals(&mut builder, "\"\" == \"é\"")?;
    compile_literals(&mut builder, "\"é\" == \"é\"")?;
    assert_eq!(
        builder.constants,
        vec![
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
            ConstValue::Symbol(SymbolId::new(2)),
            ConstValue::Symbol(SymbolId::new(1)),
        ]
    );
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from(""), Box::from("é"), Box::from("é"),])
    );
    Ok(())
}

#[test]
fn text_free_canonical_values_leave_the_table_empty() -> TestResult {
    let mut builder = SlotCompiler::new();
    compile_literals(&mut builder, "1 + 2")?;
    assert_eq!(
        builder.constants,
        vec![ConstValue::I64(1), ConstValue::I64(2)]
    );
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::<[Box<str>]>::default()
    );
    Ok(())
}

#[test]
fn wrong_receiver_rejects_before_interning_the_key() {
    let mut builder = SlotCompiler::new();
    assert_eq!(
        compile_literals(&mut builder, "has(true, \"key\")"),
        Err(CompileErrors(vec![crate::CompileError::TypeMismatch {
            field: "set.value",
            expected: "object",
            found: "boolean"
        },]))
    );
    assert!(builder.source_symbols.is_empty());
    assert_eq!(builder.constants, vec![]);
}

#[test]
fn overlong_payload_reports_next_id_without_consuming_it() -> TestResult {
    let mut builder = SlotCompiler::new();
    intern_source_text(&mut builder.source_symbols, "prefix")?;
    assert_eq!(
        intern_source_text(&mut builder.source_symbols, &"x".repeat(4097)),
        Err(crate::CompileError::Workflow(
            vb_core::WorkflowError::InvalidSymbolMetadata {
                source: vb_core::workflow::source_metadata::SourceMetadataError::SymbolTooLong {
                    id: SymbolId::new(1),
                    bytes: 4097,
                    max: 4096,
                },
            }
        ),)
    );
    assert_eq!(
        builder.source_symbols.finish()?,
        Box::from([Box::<str>::from("prefix")])
    );
    Ok(())
}
