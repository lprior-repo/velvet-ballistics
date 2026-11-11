use super::RuntimeError;
use std::error::Error;
use vb_core::value_store::InputSymbolBindingError as Binding;
use vb_core::workflow::source_metadata::SourceMetadataError as Metadata;
use vb_core::{CoreError, SymbolId};

#[path = "input_symbol_binding_tests/cases.rs"]
mod cases;
#[path = "input_symbol_binding_tests/pairs.rs"]
mod pairs;

type TestResult = Result<(), Box<dyn Error>>;

fn metadata(source: Metadata) -> RuntimeError {
    Binding::Metadata(source).into()
}

fn core(source: CoreError) -> RuntimeError {
    Binding::Core(source).into()
}

#[test]
fn bound_symbol_runtime_error_equality() -> TestResult {
    assert_metadata_equality();
    assert_core_equality();
    Ok(())
}

fn assert_metadata_equality() {
    cases::metadata_cases()
        .into_iter()
        .for_each(|(source, _, _, _)| {
            let error = metadata(source.clone());
            assert_eq!(error, error.clone());
            assert_eq!(error, metadata(source));
        });
    pairs::unequal_metadata_pairs()
        .into_iter()
        .for_each(|(a, b)| {
            assert_ne!(metadata(a), metadata(b));
        });
}

fn assert_core_equality() {
    let cause = CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    };
    assert_eq!(core(cause.clone()), core(cause.clone()));
    assert_ne!(
        core(cause.clone()),
        core(CoreError::BudgetExceeded {
            budget: "max_slots",
            limit: 4,
        })
    );
    assert_ne!(core(cause.clone()), RuntimeError::from(cause));
    assert_ne!(
        core(CoreError::AllocationFailed),
        metadata(Metadata::ValidationAllocationFailed)
    );
}

#[test]
fn bound_symbol_runtime_error_sources() -> TestResult {
    cases::metadata_cases()
        .into_iter()
        .try_for_each(|(source, _, _, _)| {
            assert_source(metadata(source.clone()), Binding::Metadata(source))
        })?;
    let cause = CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    };
    assert_source(core(cause.clone()), Binding::Core(cause))
}

fn assert_source(error: RuntimeError, expected: Binding) -> TestResult {
    let source = error.source().ok_or("missing binding source")?;
    assert_eq!(source.downcast_ref::<Binding>(), Some(&expected));
    let leaf = source.source().ok_or("missing leaf source")?;
    match &expected {
        Binding::Metadata(expected) => assert_eq!(leaf.downcast_ref::<Metadata>(), Some(expected)),
        Binding::Core(expected) => assert_eq!(leaf.downcast_ref::<CoreError>(), Some(expected)),
    }
    assert!(leaf.source().is_none());
    Ok(())
}

#[test]
fn bound_symbol_runtime_error_display() -> TestResult {
    let error = metadata(Metadata::TooManySymbols {
        count: 1_000_001,
        max: 1_000_000,
    });
    assert_display(
        error,
        "source metadata validation failed",
        "source symbol count 1000001 exceeds the arena limit 1000000",
    )?;
    let error = core(CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    });
    assert_display(
        error,
        "value store operation failed",
        "budget exceeded: max_slots limit was 3",
    )
}

fn assert_display(error: RuntimeError, binding_text: &str, leaf_text: &str) -> TestResult {
    assert_eq!(
        error.to_string(),
        format!("runtime input symbol binding failed: {binding_text}")
    );
    let source = error.source().ok_or("missing binding source")?;
    assert_eq!(source.to_string(), binding_text);
    assert_eq!(
        source.source().ok_or("missing leaf source")?.to_string(),
        leaf_text
    );
    Ok(())
}

#[test]
fn bound_symbol_runtime_error_codes_metadata() -> TestResult {
    let cases = cases::metadata_cases();
    assert_eq!(cases.len(), 11);
    cases
        .into_iter()
        .for_each(|(source, number, name, runtime)| {
            assert_codes(&metadata(source), (number, name, runtime));
        });
    Ok(())
}

fn assert_codes(error: &RuntimeError, (number, name, runtime): (u16, &str, Option<&str>)) {
    assert_eq!(error.diagnostic_code().code(), number);
    assert_eq!(error.symbolic_code().as_str(), name);
    assert_eq!(
        vb_core::HasSymbolicCode::symbolic_code(error).as_str(),
        name
    );
    assert_eq!(error.runtime_code(), runtime);
}

#[test]
fn bound_symbol_runtime_error_codes_core() -> TestResult {
    assert_bound_core_codes();
    assert_legacy_core_codes();
    Ok(())
}

fn assert_bound_core_codes() {
    let budget = CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    };
    assert_codes(
        &core(budget.clone()),
        (0x1406, "BUDGET_EXCEEDED", Some("BUDGET_EXCEEDED")),
    );
    assert_codes(
        &core(CoreError::SymbolOutOfBounds {
            symbol: SymbolId::new(4),
        }),
        (0x1311, "SYMBOL_OUT_OF_BOUNDS", None),
    );
    assert_codes(
        &core(CoreError::QueueFull),
        (0x1301, "CORE_QUEUE_FULL", Some("QUEUE_FULL")),
    );
}

fn assert_legacy_core_codes() {
    let budget = CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    };
    assert_codes(
        &RuntimeError::from(budget),
        (
            0x2008,
            "INTERNAL_INVARIANT_VIOLATION",
            Some("STORAGE_ERROR"),
        ),
    );
    assert_codes(
        &RuntimeError::from(CoreError::QueueFull),
        (0x2001, "INTERNAL_INVARIANT_VIOLATION", Some("QUEUE_FULL")),
    );
}
