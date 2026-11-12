use vb_core::SymbolId;
use vb_core::workflow::source_metadata::{SecretNameRole, SourceMetadataError as E};

pub(super) fn unequal_metadata_pairs() -> Vec<(E, E)> {
    let mut pairs = symbol_pairs();
    pairs.extend(requirement_pairs());
    pairs.extend([
        (duplicate(0, 1), duplicate(2, 1)),
        (duplicate(0, 1), duplicate(0, 3)),
        (ordered(1, 0), ordered(2, 0)),
        (ordered(1, 0), ordered(1, 3)),
        (count(5, 3), count(7, 3)),
        (count(5, 3), count(5, 4)),
        (E::SymbolIndexOverflow, E::SymbolBytesOverflow),
    ]);
    pairs
}

fn symbol_pairs() -> Vec<(E, E)> {
    vec![
        (many(10, 5), many(12, 5)),
        (many(10, 5), many(10, 8)),
        (long(0, 100, 50), long(1, 100, 50)),
        (long(0, 100, 50), long(0, 120, 50)),
        (long(0, 100, 50), long(0, 100, 60)),
    ]
}

fn many(count: usize, max: usize) -> E {
    E::TooManySymbols { count, max }
}

fn requirement_pairs() -> Vec<(E, E)> {
    use SecretNameRole::{Alias, Requirement};
    vec![
        (requirements(10, 5), requirements(12, 5)),
        (requirements(10, 5), requirements(10, 8)),
        (out_of_bounds(Alias, 0), out_of_bounds(Requirement, 0)),
        (out_of_bounds(Alias, 0), out_of_bounds(Alias, 1)),
        (empty(Alias, 0), empty(Requirement, 0)),
        (empty(Alias, 0), empty(Alias, 1)),
    ]
}

fn long(id: u32, bytes: usize, max: usize) -> E {
    E::SymbolTooLong {
        id: SymbolId::new(id),
        bytes,
        max,
    }
}

fn duplicate(first: u32, duplicate: u32) -> E {
    E::DuplicateSymbol {
        first: SymbolId::new(first),
        duplicate: SymbolId::new(duplicate),
    }
}

fn requirements(count: usize, symbols: usize) -> E {
    E::TooManyRequirements { count, symbols }
}

fn out_of_bounds(role: SecretNameRole, name: u32) -> E {
    E::RequirementNameOutOfBounds {
        role,
        name: SymbolId::new(name),
    }
}

fn empty(role: SecretNameRole, name: u32) -> E {
    E::EmptyRequirementName {
        role,
        name: SymbolId::new(name),
    }
}

fn ordered(previous: u32, name: u32) -> E {
    E::RequirementsNotStrictlyOrdered {
        previous: SymbolId::new(previous),
        name: SymbolId::new(name),
    }
}

fn count(declared: u32, actual: u32) -> E {
    E::SymbolCountMismatch { declared, actual }
}
