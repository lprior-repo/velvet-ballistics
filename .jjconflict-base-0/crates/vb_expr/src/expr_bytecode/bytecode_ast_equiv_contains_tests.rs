#[test]
fn contains_list_receiver_has_independent_ast_and_bytecode_results() -> ExprResult<()> {
    let (ast_hit, bytecode_hit) = evaluate_both("contains($d, $c)")?;
    assert_eq!(ast_hit, Ok(SlotValue::Bool(true)));
    assert_eq!(bytecode_hit, Ok(SlotValue::Bool(true)));
    let (ast_miss, bytecode_miss) = evaluate_both("contains($d, $a)")?;
    assert_eq!(ast_miss, Ok(SlotValue::Bool(false)));
    assert_eq!(bytecode_miss, Ok(SlotValue::Bool(false)));
    Ok(())
}

#[test]
fn contains_symbol_receiver_has_exact_ast_and_bytecode_errors() -> ExprResult<()> {
    let expected = Err(ExprError::TypeMismatch {
        expected: "list".into(),
        found: "symbol".into(),
    });
    let (ast_result, bytecode_result) = evaluate_both("contains($g, $h)")?;
    assert_eq!(ast_result, expected);
    assert_eq!(bytecode_result, expected);
    Ok(())
}
