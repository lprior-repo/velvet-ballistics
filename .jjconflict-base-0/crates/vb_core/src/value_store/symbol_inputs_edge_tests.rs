use super::*;

#[test]
fn compose_single_prefix_remaps_input() -> Result<(), String> {
    assert_success(
        &symbols(&["x"]),
        &checkpoint(&["a"]),
        &symbol_roots(&[0]),
        checkpoint(&["x", "a"]),
        &symbol_roots(&[1]),
    )?;
    Ok(())
}

#[test]
fn compose_remapped_list_values() -> Result<(), String> {
    let mut input = checkpoint(&["a", "b"]);
    input.lists = vec![symbol_roots(&[0, 1])];
    input.list_taints = vec![Box::from([Taint::DerivedFromSecret; 2])];
    let mut expected = checkpoint(&["x", "a", "b"]);
    expected.lists = vec![symbol_roots(&[1, 2])];
    expected.list_taints = vec![Box::from([Taint::DerivedFromSecret; 2])];
    let roots = [SlotValue::List(ListId::new(0))];
    assert_success(&symbols(&["x"]), &input, &roots, expected, &roots)?;
    Ok(())
}

#[test]
fn compose_remapped_object_keys() -> Result<(), String> {
    let mut input = checkpoint(&["key", "value"]);
    input.objects = vec![Box::from([ObjectField::clean(
        SymbolId::new(0),
        SlotValue::Symbol(SymbolId::new(1)),
    )])];
    let mut expected = checkpoint(&["x", "key", "value"]);
    expected.objects = vec![Box::from([ObjectField::clean(
        SymbolId::new(1),
        SlotValue::Symbol(SymbolId::new(2)),
    )])];
    let roots = [SlotValue::Object(ObjectId::new(0))];
    assert_success(&symbols(&["x"]), &input, &roots, expected, &roots)?;
    Ok(())
}

#[test]
fn compose_empty_compiler_symbols() -> Result<(), String> {
    let input = checkpoint(&["a"]);
    assert_success(
        &[],
        &input,
        &symbol_roots(&[0]),
        checkpoint(&["a"]),
        &symbol_roots(&[0]),
    )?;
    Ok(())
}

#[test]
fn compose_empty_input_symbols() -> Result<(), String> {
    assert_success(
        &symbols(&["x"]),
        &checkpoint(&[]),
        &[],
        checkpoint(&["x"]),
        &[],
    )?;
    Ok(())
}

#[test]
fn compose_duplicate_input_symbols() -> Result<(), String> {
    assert_success(
        &symbols(&["x"]),
        &checkpoint(&["a", "a"]),
        &symbol_roots(&[0, 1]),
        checkpoint(&["x", "a"]),
        &symbol_roots(&[1, 1]),
    )?;
    Ok(())
}

#[test]
fn compose_empty_compiler_symbol_text() -> Result<(), String> {
    assert_success(
        &symbols(&[""]),
        &checkpoint(&["a"]),
        &symbol_roots(&[0]),
        checkpoint(&["", "a"]),
        &symbol_roots(&[1]),
    )?;
    Ok(())
}

#[test]
fn compose_empty_input_symbol_text() -> Result<(), String> {
    assert_success(
        &symbols(&["x"]),
        &checkpoint(&[""]),
        &symbol_roots(&[0]),
        checkpoint(&["x", ""]),
        &symbol_roots(&[1]),
    )?;
    Ok(())
}

#[test]
fn compose_distinct_prefix_remaps_both_roots() -> Result<(), String> {
    assert_success(
        &symbols(&["x", "y"]),
        &checkpoint(&["a", "b"]),
        &symbol_roots(&[0, 1]),
        checkpoint(&["x", "y", "a", "b"]),
        &symbol_roots(&[2, 3]),
    )?;
    Ok(())
}

#[test]
fn compose_exact_limit_compiler_symbol() -> Result<(), String> {
    let text = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE);
    assert_success(
        &symbols(&[&text]),
        &checkpoint(&["a"]),
        &symbol_roots(&[0]),
        checkpoint(&[&text, "a"]),
        &symbol_roots(&[1]),
    )?;
    Ok(())
}
