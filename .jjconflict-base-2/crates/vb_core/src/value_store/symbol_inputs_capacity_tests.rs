use super::*;

fn unique_symbols(count: usize) -> Vec<Box<str>> {
    (0..count)
        .map(|index| format!("symbol-{index}").into_boxed_str())
        .collect()
}

fn capacity_checkpoint(count: usize) -> ValueStoreCheckpointV1 {
    let mut input = checkpoint(&[]);
    input.symbols = unique_symbols(count);
    input
}

fn last_symbol_index(count: usize) -> Result<u32, String> {
    let last = count.checked_sub(1).ok_or("empty capacity fixture")?;
    u32::try_from(last).map_err(|error| format!("{error:?}"))
}

#[test]
fn compose_rejects_actual_compiler_symbol_count_too_high() -> Result<(), String> {
    let count = MAX_VALUES_PER_RUN.checked_add(1).ok_or("count overflow")?;
    let compiler = unique_symbols(count);
    assert_error(
        &compiler,
        &checkpoint(&["a"]),
        &symbol_roots(&[0]),
        InputSymbolBindingError::Metadata(SourceMetadataError::TooManySymbols {
            count,
            max: MAX_VALUES_PER_RUN,
        }),
    )
}

#[test]
fn compose_exact_compiler_capacity_succeeds() -> Result<(), String> {
    let compiler = unique_symbols(MAX_VALUES_PER_RUN);
    assert_success(
        &compiler,
        &checkpoint(&[]),
        &[],
        capacity_checkpoint(MAX_VALUES_PER_RUN),
        &[],
    )?;
    Ok(())
}

#[test]
fn compose_duplicate_at_capacity_reuses_existing_symbol() -> Result<(), String> {
    let compiler = unique_symbols(MAX_VALUES_PER_RUN);
    let last = last_symbol_index(MAX_VALUES_PER_RUN)?;
    let last_text = format!("symbol-{last}");
    assert_success(
        &compiler,
        &checkpoint(&["symbol-0", &last_text, "symbol-0"]),
        &symbol_roots(&[0, 1, 2]),
        capacity_checkpoint(MAX_VALUES_PER_RUN),
        &symbol_roots(&[0, last, 0]),
    )?;
    Ok(())
}

#[test]
fn compose_combined_unique_capacity_overflow_rejects() -> Result<(), String> {
    let compiler = unique_symbols(MAX_VALUES_PER_RUN);
    let count = MAX_VALUES_PER_RUN.checked_add(1).ok_or("count overflow")?;
    assert_error(
        &compiler,
        &checkpoint(&["new-unique-symbol"]),
        &symbol_roots(&[0]),
        InputSymbolBindingError::Metadata(SourceMetadataError::TooManySymbols {
            count,
            max: MAX_VALUES_PER_RUN,
        }),
    )
}

#[test]
fn compose_duplicate_input_tail_at_capacity_succeeds() -> Result<(), String> {
    let mut input = capacity_checkpoint(MAX_VALUES_PER_RUN);
    input.symbols.push("symbol-0".into());
    let duplicate = u32::try_from(MAX_VALUES_PER_RUN).map_err(|error| format!("{error:?}"))?;
    assert_success(
        &[],
        &input,
        &symbol_roots(&[duplicate]),
        capacity_checkpoint(MAX_VALUES_PER_RUN),
        &symbol_roots(&[0]),
    )?;
    Ok(())
}
