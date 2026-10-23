//! Tests the real standalone interner, without installing runtime metadata.

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn empty_table_finishes_without_a_placeholder_symbol() -> TestResult {
    let table = SourceSymbolTable::new();
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
    assert_eq!(table.symbol_bytes(), 0);
    assert!(table.finish()?.is_empty());
    Ok(())
}

#[test]
fn first_definition_order_is_stable_and_duplicates_share_an_id() -> TestResult {
    let mut table = SourceSymbolTable::new();
    assert_eq!(table.intern("zeta")?, SymbolId::new(0));
    assert_eq!(table.intern("alpha")?, SymbolId::new(1));
    assert_eq!(table.intern("zeta")?, SymbolId::new(0));
    assert_eq!(table.intern("")?, SymbolId::new(2));
    assert_eq!(table.intern("alpha")?, SymbolId::new(1));
    assert_eq!(table.len(), 3);
    assert_eq!(table.symbol_bytes(), 9);
    let expected =
        vec![Box::<str>::from("zeta"), Box::from("alpha"), Box::from("")].into_boxed_slice();
    assert_eq!(table.finish()?, expected);
    Ok(())
}

#[test]
fn fresh_hash_tables_produce_identical_payload_order() -> TestResult {
    let definitions = ["last", "first", "middle", "last", "first", ""];
    let mut expected = None;
    for _ in 0..32 {
        let mut table = SourceSymbolTable::new();
        for text in definitions {
            table.intern(text)?;
        }
        let symbols = table.finish()?;
        if let Some(previous) = &expected {
            assert_eq!(&symbols, previous);
        } else {
            expected = Some(symbols);
        }
    }
    Ok(())
}

#[test]
fn exact_unicode_bytes_are_interned_without_normalization() -> TestResult {
    let mut table = SourceSymbolTable::new();
    assert_eq!(table.intern("é")?, SymbolId::new(0));
    assert_eq!(table.intern("e\u{301}")?, SymbolId::new(1));
    assert_eq!(table.intern("é")?, SymbolId::new(0));
    assert_eq!(table.symbol_bytes(), 5);
    let expected = vec![Box::<str>::from("é"), Box::from("e\u{301}")].into_boxed_slice();
    assert_eq!(table.finish()?, expected);
    Ok(())
}

#[test]
fn exact_symbol_byte_boundary_is_accepted() -> TestResult {
    let text = "x".repeat(MAX_SYMBOL_BYTES_PER_VALUE);
    let mut table = SourceSymbolTable::new();
    assert_eq!(table.intern(&text)?, SymbolId::new(0));
    assert_eq!(
        table.symbol_bytes(),
        u64::try_from(MAX_SYMBOL_BYTES_PER_VALUE)?
    );
    assert_eq!(
        table.finish()?.first().map(Box::as_ref),
        Some(text.as_str())
    );
    Ok(())
}

#[test]
fn oversize_addition_is_atomic_and_does_not_consume_an_id() -> TestResult {
    let mut table = SourceSymbolTable::new();
    table.intern("before")?;
    let before_bytes = table.symbol_bytes();
    let excessive = MAX_SYMBOL_BYTES_PER_VALUE
        .checked_add(1)
        .ok_or("size overflow")?;
    assert_eq!(
        table.intern(&"x".repeat(excessive)),
        Err(SourceSymbolError::SymbolTooLong {
            actual: excessive,
            limit: MAX_SYMBOL_BYTES_PER_VALUE,
        })
    );
    assert_eq!(table.len(), 1);
    assert_eq!(table.symbol_bytes(), before_bytes);
    assert_eq!(table.intern("before")?, SymbolId::new(0));
    assert_eq!(table.intern("after")?, SymbolId::new(1));
    let expected = vec![Box::<str>::from("before"), Box::from("after")].into_boxed_slice();
    assert_eq!(table.finish()?, expected);
    Ok(())
}

#[test]
fn unicode_limit_counts_bytes_not_characters() -> TestResult {
    let mut table = SourceSymbolTable::new();
    let allowed = "é".repeat(
        MAX_SYMBOL_BYTES_PER_VALUE
            .checked_div(2)
            .ok_or("zero divisor")?,
    );
    table.intern(&allowed)?;
    let rejected = format!("{allowed}é");
    assert!(matches!(
        table.intern(&rejected),
        Err(SourceSymbolError::SymbolTooLong { .. })
    ));
    assert_eq!(table.len(), 1);
    assert_eq!(table.symbol_bytes(), u64::try_from(allowed.len())?);
    Ok(())
}

#[test]
fn checked_count_boundary_matches_existing_runtime_limit() -> TestResult {
    let last = MAX_VALUES_PER_RUN
        .checked_sub(1)
        .ok_or("empty symbol domain")?;
    assert_eq!(
        checked_next_symbol_id(last)?,
        SymbolId::new(u32::try_from(last)?)
    );
    for count in [MAX_VALUES_PER_RUN, usize::MAX] {
        assert_eq!(
            checked_next_symbol_id(count),
            Err(SourceSymbolError::SymbolCountLimit {
                count,
                limit: MAX_VALUES_PER_RUN,
            })
        );
    }
    Ok(())
}

#[test]
fn byte_total_uses_checked_measurement_without_an_aggregate_ceiling() -> TestResult {
    assert_eq!(checked_symbol_bytes(u64::MAX, 0)?, u64::MAX);
    assert_eq!(
        checked_symbol_bytes(u64::MAX, 1),
        Err(SourceSymbolError::SymbolBytesOverflow)
    );
    let mut table = SourceSymbolTable::new();
    // 257 allowed symbols cross 1 MiB; no new aggregate cap is introduced.
    for index in 0..257 {
        let prefix = format!("s{index:06}");
        let padding = MAX_SYMBOL_BYTES_PER_VALUE
            .checked_sub(prefix.len())
            .ok_or("prefix too long")?;
        table.intern(&format!("{prefix}{}", "x".repeat(padding)))?;
    }
    assert_eq!(table.len(), 257);
    assert_eq!(table.symbol_bytes(), 1_052_672);
    assert_eq!(table.finish()?.len(), 257);
    Ok(())
}

#[test]
fn full_table_rejects_new_text_but_still_resolves_existing_text() -> TestResult {
    let mut table = SourceSymbolTable::new();
    for index in 0..MAX_VALUES_PER_RUN {
        assert_eq!(
            table.intern(&format!("s{index}"))?,
            SymbolId::new(u32::try_from(index)?)
        );
    }
    let before_bytes = table.symbol_bytes();
    assert_eq!(
        table.intern("overflow"),
        Err(SourceSymbolError::SymbolCountLimit {
            count: MAX_VALUES_PER_RUN,
            limit: MAX_VALUES_PER_RUN,
        })
    );
    assert_eq!(table.len(), MAX_VALUES_PER_RUN);
    assert_eq!(table.symbol_bytes(), before_bytes);
    assert_eq!(table.intern("s0")?, SymbolId::new(0));
    Ok(())
}
