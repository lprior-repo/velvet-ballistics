#[test]
fn compile_and_parse_ast_reject_non_boolean_choose_condition() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: type_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 2
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}

#[test]
fn compile_accepts_initialized_boolean_slot_choose_condition() -> Result<(), String> {
    let source = initialized_boolean_slot_choose_source();

    ensure_compile_and_parse_ok(source)?;
    ensure_expression(
        choose_expression(source)?,
        AstExpression::Slot(vb_core::SlotIdx::new(0)),
        "initialized boolean slot AST condition mismatch",
    )?;
    ensure_choose_slot_node(source)
}

#[test]
fn compile_and_parse_ast_accept_boolean_literal_choose_condition() -> Result<(), String> {
    let source = literal_boolean_choose_source();

    ensure_compile_and_parse_ok(source)?;
    ensure_expression(
        choose_expression(source)?,
        AstExpression::Literal(AstValue::Bool(true)),
        "boolean literal AST condition mismatch",
    )?;
    ensure_literal_choose_node(source)
}

#[test]
fn compile_and_parse_ast_reject_uninitialized_choose_slot() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: choose_case
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: 1
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_unknown_choose_slot)
}

#[test]
fn compile_and_parse_ast_reject_choose_slot_index_out_of_range_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: choose_case
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: 65536
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: null
"#;

    ensure_pair(source, ensure_slot_index_out_of_range)
}

#[test]
fn compile_and_parse_ast_reject_missing_finish_slot() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: finish_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
