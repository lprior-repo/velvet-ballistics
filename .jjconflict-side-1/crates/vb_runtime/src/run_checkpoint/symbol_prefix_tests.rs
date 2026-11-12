use super::validate_bound_symbol_prefix;
use crate::run_checkpoint::RunCheckpointV1Error;
use crate::run_checkpoint::symbol_prefix_fixture::{
    TestResult, legacy_workflow, prefix_workflow, symbol_table,
};
use proptest::prelude::*;

#[test]
fn bound_symbol_runtime_prefix_none_and_bound_empty() -> TestResult {
    let legacy = legacy_workflow()?;
    let empty = prefix_workflow(Vec::new())?;
    let table = symbol_table(&["tail", "tail"]);
    let before = table.clone();
    assert_eq!(legacy.symbol_payloads().map(|items| items.len()), None);
    assert_eq!(empty.symbol_payloads().map(|items| items.len()), Some(0));
    assert_eq!(validate_bound_symbol_prefix(&legacy, &table), Ok(()));
    assert_eq!(validate_bound_symbol_prefix(&empty, &table), Ok(()));
    assert_eq!(table, before);
    Ok(())
}

#[test]
fn bound_symbol_runtime_prefix_exact_utf8_and_duplicate_tail() -> TestResult {
    let workflow = prefix_workflow(vec!["key".into(), "é".into(), "".into()])?;
    let table = symbol_table(&["key", "é", "", "key", "tail", "key"]);
    let before = table.clone();
    assert_eq!(validate_bound_symbol_prefix(&workflow, &table), Ok(()));
    assert_eq!(table, before);
    let different = symbol_table(&["key", "e\u{301}", "", "key", "tail", "key"]);
    assert_eq!(
        validate_bound_symbol_prefix(&workflow, &different),
        Err(RunCheckpointV1Error::SymbolPrefixMismatch { index: 1 })
    );
    Ok(())
}

#[test]
fn bound_symbol_runtime_prefix_first_wrong_or_missing_position() -> TestResult {
    let workflow = prefix_workflow(vec!["key".into(), "second".into(), "third".into()])?;
    for offset in 0_usize..3 {
        let index = u32::try_from(offset)?;
        let mut wrong = symbol_table(&["key", "second", "third", "tail", "key"]);
        *wrong
            .symbols
            .get_mut(offset)
            .ok_or("missing fixture symbol")? = "wrong".into();
        assert_prefix_rejection(&workflow, wrong, index)?;
        let mut missing = symbol_table(&["key", "second", "third"]);
        missing.symbols.truncate(offset);
        assert_prefix_rejection(&workflow, missing, index)?;
    }
    Ok(())
}

fn prefix_property_case(length: usize, tails: usize, choice: usize, missing: bool) -> TestResult {
    let prefix: Vec<Box<str>> = (0..length).map(|i| format!("prefix_{i}").into()).collect();
    let workflow = prefix_workflow(prefix.clone())?;
    let mut table = symbol_table(&[]);
    table.symbols = prefix;
    table
        .symbols
        .extend((0..tails).map(|_| Box::from("duplicate-tail")));
    let before = table.clone();
    assert_eq!(validate_bound_symbol_prefix(&workflow, &table), Ok(()));
    assert_eq!(table, before);
    if length != 0 {
        assert_corrupted_prefix(
            &workflow,
            table,
            choice.min(length.saturating_sub(1)),
            missing,
        )?;
    }
    Ok(())
}

fn assert_corrupted_prefix(
    workflow: &vb_core::CompiledWorkflow,
    mut table: vb_core::value_store::ValueStoreCheckpointV1,
    offset: usize,
    missing: bool,
) -> TestResult {
    if missing {
        table.symbols.truncate(offset);
    } else {
        *table
            .symbols
            .get_mut(offset)
            .ok_or("missing mutation symbol")? = "wrong".into();
    }
    let before = table.clone();
    assert_eq!(
        validate_bound_symbol_prefix(workflow, &table),
        Err(RunCheckpointV1Error::SymbolPrefixMismatch {
            index: u32::try_from(offset)?
        })
    );
    assert_eq!(table, before);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn bound_symbol_runtime_prefix_property(
        length in 0_usize..9, tails in 0_usize..9,
        choice in 0_usize..9, missing in any::<bool>(),
    ) {
        prefix_property_case(length, tails, choice, missing)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
    }
}

fn assert_prefix_rejection(
    workflow: &vb_core::CompiledWorkflow,
    table: vb_core::value_store::ValueStoreCheckpointV1,
    index: u32,
) -> TestResult {
    let before = table.clone();
    assert_eq!(
        validate_bound_symbol_prefix(workflow, &table),
        Err(RunCheckpointV1Error::SymbolPrefixMismatch { index })
    );
    assert_eq!(table, before);
    Ok(())
}
