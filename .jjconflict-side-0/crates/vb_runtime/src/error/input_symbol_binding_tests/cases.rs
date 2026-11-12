use vb_core::SymbolId;
use vb_core::workflow::source_metadata::{SecretNameRole, SourceMetadataError as E};

pub(super) type CodeCase = (E, u16, &'static str, Option<&'static str>);

pub(super) fn metadata_cases() -> Vec<CodeCase> {
    let mut cases = budget_cases();
    cases.extend(invariant_cases());
    cases.extend(invalid_cases().map(|error| {
        (
            error,
            0x1307,
            "INVALID_COMPILED_WORKFLOW_CORE",
            Some("INVALID_COMPILED_WORKFLOW"),
        )
    }));
    cases
}

fn invariant_cases() -> [CodeCase; 3] {
    [
        (
            E::SymbolIndexOverflow,
            0x1309,
            "INTERNAL_INVARIANT_VIOLATION",
            Some("INTERNAL_INVARIANT_VIOLATION"),
        ),
        (
            E::SymbolBytesOverflow,
            0x1309,
            "INTERNAL_INVARIANT_VIOLATION",
            Some("INTERNAL_INVARIANT_VIOLATION"),
        ),
        (
            E::ValidationAllocationFailed,
            0x1303,
            "ALLOCATION_FAILED",
            None,
        ),
    ]
}

fn budget_cases() -> Vec<CodeCase> {
    vec![
        (
            E::TooManySymbols { count: 10, max: 5 },
            0x1406,
            "BUDGET_EXCEEDED",
            Some("BUDGET_EXCEEDED"),
        ),
        (
            E::SymbolTooLong {
                id: SymbolId::new(0),
                bytes: 100,
                max: 50,
            },
            0x1406,
            "BUDGET_EXCEEDED",
            Some("BUDGET_EXCEEDED"),
        ),
    ]
}

fn invalid_cases() -> impl Iterator<Item = E> {
    [
        E::DuplicateSymbol {
            first: SymbolId::new(0),
            duplicate: SymbolId::new(1),
        },
        E::TooManyRequirements {
            count: 10,
            symbols: 5,
        },
        E::RequirementNameOutOfBounds {
            role: SecretNameRole::Alias,
            name: SymbolId::new(0),
        },
    ]
    .into_iter()
    .chain(invalid_order_cases())
}

fn invalid_order_cases() -> impl Iterator<Item = E> {
    [
        E::EmptyRequirementName {
            role: SecretNameRole::Requirement,
            name: SymbolId::new(0),
        },
        E::RequirementsNotStrictlyOrdered {
            previous: SymbolId::new(1),
            name: SymbolId::new(0),
        },
        E::SymbolCountMismatch {
            declared: 5,
            actual: 3,
        },
    ]
    .into_iter()
}
