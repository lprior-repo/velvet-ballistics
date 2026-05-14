    ensure_choose_rejects_type(
        &literal_choose_condition_source("\n        value: true"),
        "object",
    )
}

#[test]
fn compile_and_parse_ast_reject_initialized_non_boolean_slot_conditions() -> Result<(), String> {
    let text_source = initialized_slot_condition_source("public");
    let null_source = initialized_slot_condition_source("null");
    let list_source = initialized_slot_condition_source("[true]");
    let object_source = initialized_slot_condition_source("{ value: true }");

    ensure_choose_rejects_type(&text_source, "text")?;
    ensure_choose_rejects_type(&null_source, "null")?;
    ensure_choose_rejects_type(&list_source, "list")?;
    ensure_choose_rejects_type(&object_source, "object")
}

#[test]
fn compile_and_parse_ast_reject_secret_object_finish_result_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: done
    finish:
      result:
        token: $secrets.token
"#;

    ensure_secret_result_pair(source)
}

#[test]
fn compile_and_parse_ast_reject_uninitialized_forward_finish_slot() -> Result<(), String> {
    ensure_pair(forward_finish_slot_source(), ensure_forward_finish_slot)
}

#[test]
fn parse_ast_accepts_clean_literal_finish_results() -> Result<(), String> {
    ensure_expression(
        finish_expression(&finish_result_fragment_source(" '\"public\"'"))?,
        parsed_text_expression("public"),
        "text finish literal was not retained exactly",
    )?;
    ensure_expression(
        finish_expression(&finish_result_fragment_source("\n        - public"))?,
        AstExpression::Literal(AstValue::Sequence(vec![AstValue::Text("public".into())])),
        "list finish literal was not retained exactly",
    )?;
    ensure_expression(
        finish_expression(&finish_result_fragment_source("\n        value: public"))?,
        AstExpression::Literal(AstValue::Mapping(vec![AstMapEntry {
            name: "value".into(),
            value: AstValue::Text("public".into()),
            mark: None,
        }])),
        "object finish literal was not retained exactly",
    )
}

#[test]
fn compile_and_parse_ast_accept_supported_scalar_finish_literals() -> Result<(), String> {
    ensure_supported_scalar_finish_const("null", ConstValue::Null)?;
    ensure_supported_scalar_finish_const("true", ConstValue::Bool(true))?;
    ensure_supported_scalar_finish_const("false", ConstValue::Bool(false))?;
    ensure_supported_scalar_finish_const("42", ConstValue::I64(42))?;
    ensure_supported_scalar_finish_const("-7", ConstValue::I64(-7))
}

#[test]
fn finish_result_zero_keeps_current_slot_zero_semantics() -> Result<(), String> {
    let zero_source = finish_literal_source("0");

    ensure_pair(&zero_source, ensure_unknown_finish_slot)
}

#[test]
fn compile_rejects_unsupported_finish_literals_with_exact_error() -> Result<(), String> {
    let text_source = finish_literal_source("'\"public\"'");
    let list_source = finish_literal_source("[public]");
    let object_source = finish_literal_source("{ value: public }");

    ensure_compile_unsupported_constant(&text_source)?;
    ensure_compile_unsupported_constant(&list_source)?;
    ensure_compile_unsupported_constant(&object_source)
}

#[test]
fn parse_ast_accepts_supported_scalar_finish_literals() -> Result<(), String> {
    ensure_supported_scalar_finish_asts()
}

#[test]
fn reference_errors_preempt_type_taint_errors() -> Result<(), String> {
    ensure_pair(reference_preempt_source(), ensure_reference_error)
