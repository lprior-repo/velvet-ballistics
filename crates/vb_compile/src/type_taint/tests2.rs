
    ensure_pair(source, ensure_unknown_finish_slot)
}

#[test]
fn parse_ast_rejects_finish_slot_index_out_of_range_exactly() -> Result<(), String> {
    let source = large_finish_slot_source();
    let compiler = YamlCompiler::new(YamlLimits {
        max_source_bytes: 4_000_000,
        max_depth: 64,
        max_nodes: 500_000,
        max_sequence_len: 70_000,
        max_mapping_entries: 1_024,
        max_scalar_bytes: 65_536,
    });

    match compiler.parse_ast(&source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(errors) => match errors.0.into_iter().next() {
            Some(error) => ensure_slot_index_out_of_range(error),
            None => Err("parse_ast returned empty CompileErrors".to_owned()),
        },
    }
}

#[test]
fn validator_rejects_secret_tainted_finish_result() -> Result<(), String> {
    let ast = secret_tainted_finish_ast();

    match validate_workflow_ast(&ast) {
        Ok(()) => Err("validator unexpectedly accepted secret result".to_owned()),
        Err(errors) => match errors.0.into_iter().next() {
            Some(error) => ensure_secret_result(error),
            None => Err("validate_workflow_ast returned empty CompileErrors".to_owned()),
        },
    }
}

#[test]
fn compile_and_parse_ast_reject_secret_reference_finish_result_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: done
    finish:
      result: $secrets.token
"#;

    ensure_secret_result_pair(source)
}

#[test]
fn compile_and_parse_ast_reject_secret_slot_finish_result_exactly() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: taint_case
when:
  manual: {}
secrets:
  token: TOKEN
steps:
  - id: capture
    save:
      value: $secrets.token
  - id: done
    finish:
      result: 0
"#;

    ensure_secret_result_pair(source)
}

#[test]
fn compile_and_parse_ast_reject_nested_secret_slot_finish_results() -> Result<(), String> {
    ensure_secret_result_pair(nested_secret_list_finish_source())?;
    ensure_secret_result_pair(nested_secret_object_finish_source())
}

#[test]
fn parse_ast_accepts_clean_public_finish_references_exactly() -> Result<(), String> {
    ensure_expression(
        finish_expression(clean_input_finish_source())?,
        parsed_reference_expression("$input.user"),
        "input finish reference was not retained exactly",
    )?;
    ensure_expression(
        finish_expression(clean_vars_finish_source())?,
        parsed_reference_expression("$vars.label"),
        "vars finish reference was not retained exactly",
    )
}

#[test]
fn compile_and_parse_ast_reject_non_boolean_literal_choose_conditions() -> Result<(), String> {
    ensure_choose_rejects_type(&literal_choose_condition_source(" '\"public\"'"), "text")?;
    ensure_choose_rejects_type(&literal_choose_condition_source(" null"), "null")?;
    ensure_choose_rejects_type(&literal_choose_condition_source("\n        - true"), "list")?;
