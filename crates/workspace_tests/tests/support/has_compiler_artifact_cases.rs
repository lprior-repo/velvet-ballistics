use crate::has_compiler_artifacts::{
    assert_ast_has, assert_compiled_has, assert_single_any_slot_has, compile_artifacts_with_inputs,
};
use vb_core::InputSlotKind;

#[test]
fn compile_accepts_any_receiver_text_key() -> Result<(), String> {
    let artifacts = compile_artifacts_with_inputs(
        &[("obj", "any"), ("key", "text")],
        "has($input.obj, $input.key)",
    )
    .map_err(|errors| format!("canonical compilation failed: {errors:?}"))?;
    assert_compiled_has(
        &artifacts,
        &[
            ("obj", "any", InputSlotKind::Any),
            ("key", "text", InputSlotKind::Symbol),
        ],
    )
}

#[test]
fn compile_accepts_object_receiver_any_key() -> Result<(), String> {
    let artifacts = compile_artifacts_with_inputs(
        &[("obj", "object"), ("key", "any")],
        "has($input.obj, $input.key)",
    )
    .map_err(|errors| format!("canonical compilation failed: {errors:?}"))?;
    assert_compiled_has(
        &artifacts,
        &[
            ("obj", "object", InputSlotKind::Object),
            ("key", "any", InputSlotKind::Any),
        ],
    )
}

#[test]
fn compile_accepts_any_receiver_any_key() -> Result<(), String> {
    let artifacts = compile_artifacts_with_inputs(
        &[("obj", "any"), ("key", "any")],
        "has($input.obj, $input.key)",
    )
    .map_err(|errors| format!("canonical compilation failed: {errors:?}"))?;
    assert_compiled_has(
        &artifacts,
        &[
            ("obj", "any", InputSlotKind::Any),
            ("key", "any", InputSlotKind::Any),
        ],
    )
}

#[test]
fn compile_accepts_same_any_slot_in_both_has_positions() -> Result<(), String> {
    let artifacts =
        compile_artifacts_with_inputs(&[("value", "any")], "has($input.value, $input.value)")
            .map_err(|errors| format!("canonical compilation failed: {errors:?}"))?;
    assert_single_any_slot_has(&artifacts)
}

#[test]
fn parse_ast_accepts_any_any_inputs() -> Result<(), String> {
    let ast = parse_ast_with_inputs(
        &[("obj", "any"), ("key", "any")],
        "has($input.obj, $input.key)",
    )
    .map_err(|errors| format!("parse-AST failed: {errors:?}"))?;
    assert_ast_has(
        &ast,
        &[
            ("obj", "any", InputSlotKind::Any),
            ("key", "any", InputSlotKind::Any),
        ],
        &["$input.obj", "$input.key"],
        None,
    )
}

#[test]
fn parse_ast_accepts_any_receiver_text_key() -> Result<(), String> {
    let ast = parse_ast_with_inputs(
        &[("obj", "any"), ("key", "text")],
        "has($input.obj, $input.key)",
    )
    .map_err(|errors| format!("parse-AST failed: {errors:?}"))?;
    assert_ast_has(
        &ast,
        &[
            ("obj", "any", InputSlotKind::Any),
            ("key", "text", InputSlotKind::Symbol),
        ],
        &["$input.obj", "$input.key"],
        None,
    )
}

#[test]
fn parse_ast_accepts_object_receiver_any_key() -> Result<(), String> {
    let ast = parse_ast_with_inputs(
        &[("obj", "object"), ("key", "any")],
        "has($input.obj, $input.key)",
    )
    .map_err(|errors| format!("parse-AST failed: {errors:?}"))?;
    assert_ast_has(
        &ast,
        &[
            ("obj", "object", InputSlotKind::Object),
            ("key", "any", InputSlotKind::Any),
        ],
        &["$input.obj", "$input.key"],
        None,
    )
}

#[test]
fn parse_ast_preserves_no_inputs() -> Result<(), String> {
    let ast = parse_ast_with_inputs(&[], "has()").map_err(|errors| format!("{errors:?}"))?;
    assert_ast_has(&ast, &[], &[], None)
}

#[test]
fn parse_ast_preserves_obj_input_declared_any() -> Result<(), String> {
    let ast = parse_ast_with_inputs(&[("obj", "any")], "has($input.obj)")
        .map_err(|errors| format!("{errors:?}"))?;
    assert_ast_has(
        &ast,
        &[("obj", "any", InputSlotKind::Any)],
        &["$input.obj"],
        None,
    )
}

#[test]
fn parse_ast_preserves_obj_key_value_inputs() -> Result<(), String> {
    let ast = parse_ast_with_inputs(
        &[("obj", "any"), ("key", "any")],
        "has($input.obj, $input.key, 1)",
    )
    .map_err(|errors| format!("{errors:?}"))?;
    assert_ast_has(
        &ast,
        &[
            ("obj", "any", InputSlotKind::Any),
            ("key", "any", InputSlotKind::Any),
        ],
        &["$input.obj", "$input.key"],
        Some(1),
    )
}
