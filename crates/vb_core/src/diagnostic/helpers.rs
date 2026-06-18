//! Lookup helpers for the canonical code registry.
//!
//! These functions are pure, allocation-free, and operate only on
//! `CODE_REGISTRY` from [`super::codes`].  They are used by both
//! `codes.rs` (for cross-referencing) and `types.rs` (for type methods).

use super::codes::CODE_REGISTRY;

/// Looks up a symbolic code name and returns its numeric encoding.
///
/// Returns `None` if the symbolic name is not registered.
#[must_use]
pub fn symbolic_to_numeric(symbolic: &str) -> Option<u16> {
    for entry in CODE_REGISTRY {
        if entry.symbolic == symbolic {
            return Some(entry.numeric);
        }
    }
    None
}

/// Looks up a numeric code and returns its symbolic name.
///
/// Returns `None` if the numeric code is not in the registry.
#[must_use]
pub fn numeric_to_symbolic(numeric: u16) -> Option<&'static str> {
    for entry in CODE_REGISTRY {
        if entry.numeric == numeric {
            return Some(entry.symbolic);
        }
    }
    None
}

/// Looks up a numeric code and returns the corresponding symbolic string.
///
/// Returns `None` if the numeric code is not registered.
#[must_use]
pub fn numeric_to_symbolic_str(numeric: u16) -> Option<&'static str> {
    numeric_to_symbolic(numeric)
}

/// Returns `true` when the given symbolic string is registered in
/// [`CODE_REGISTRY`](super::codes::CODE_REGISTRY).
#[must_use]
pub fn is_registered_symbolic(name: &str) -> bool {
    symbolic_to_numeric(name).is_some()
}

/// Returns `true` when the given numeric code is registered in
/// [`CODE_REGISTRY`](super::codes::CODE_REGISTRY).
#[must_use]
pub fn is_registered_numeric(code: u16) -> bool {
    numeric_to_symbolic(code).is_some()
}

/// Classifies a numeric code into its [`CodeCategory`](super::codes::CodeCategory)
/// by consulting the [`CODE_REGISTRY`](super::codes::CODE_REGISTRY) first,
/// falling back to high-byte heuristics when the numeric code is not yet
/// registered.
///
/// This ensures that registry entries with explicit categories (such as
/// `CodeCategory::Internal` for `INTERNAL_INVARIANT_VIOLATION` at
/// `0x1309`) are correctly classified instead of being misclassified by
/// the high byte alone.
#[must_use]
pub fn category_from_numeric(numeric: u16) -> super::codes::CodeCategory {
    // 1. Consult registry for the authoritative category.
    for entry in CODE_REGISTRY {
        if entry.numeric == numeric {
            return entry.category;
        }
    }
    // 2. Fall back to high-byte heuristics for unregistered codes.
    let high_byte = numeric.wrapping_shr(8) & 0xFF_u16;
    match high_byte {
        0x01 => super::codes::CodeCategory::Schema,
        0x02 => super::codes::CodeCategory::Reference,
        0x03 => super::codes::CodeCategory::ControlFlow,
        0x04 => super::codes::CodeCategory::TypeTaint,
        0x05 => super::codes::CodeCategory::Gate,
        0x06 => super::codes::CodeCategory::ContractDiscovery,
        0x10 => super::codes::CodeCategory::Compilation,
        0x11 => super::codes::CodeCategory::WorkflowIr,
        0x12 => super::codes::CodeCategory::Expression,
        0x13 => super::codes::CodeCategory::Accessor,
        0x14 => super::codes::CodeCategory::Lowering,
        0x15 => super::codes::CodeCategory::Lifecycle,
        0x20 => super::codes::CodeCategory::Storage,
        0x30 => super::codes::CodeCategory::Runtime,
        0x32 => super::codes::CodeCategory::Ipc,
        0x33 => super::codes::CodeCategory::Lifecycle,
        0x40 => super::codes::CodeCategory::RuntimeBoundary,
        _ => super::codes::CodeCategory::Internal, // unregistered high bytes → Internal
    }
}

/// Returns `true` when the numeric code is registered in
/// [`CODE_REGISTRY`](super::codes::CODE_REGISTRY).
#[must_use]
pub(crate) fn is_supported_code(code: u16) -> bool {
    is_registered_numeric(code)
}

/// Parses a single hexadecimal character into a `u16` digit.
pub(super) fn parse_hex_digit(value: Option<char>) -> Result<u16, super::types::DiagnosticCodeParseError> {
    let Some(character) = value else {
        return Err(super::types::DiagnosticCodeParseError::InvalidFormat);
    };
    let Some(digit) = character.to_digit(16) else {
        return Err(super::types::DiagnosticCodeParseError::InvalidFormat);
    };
    u16::try_from(digit).map_err(|_| super::types::DiagnosticCodeParseError::InvalidFormat)
}

/// Packs four hexadecimal digits into a `u16` diagnostic code.
pub(super) fn pack_digits(
    first: u16,
    second: u16,
    third: u16,
    fourth: u16,
) -> Result<u16, super::types::DiagnosticCodeParseError> {
    let first_shifted = first
        .checked_shl(12)
        .ok_or(super::types::DiagnosticCodeParseError::InvalidFormat)?;
    let second_shifted = second
        .checked_shl(8)
        .ok_or(super::types::DiagnosticCodeParseError::InvalidFormat)?;
    let third_shifted = third
        .checked_shl(4)
        .ok_or(super::types::DiagnosticCodeParseError::InvalidFormat)?;
    first_shifted
        .checked_add(second_shifted)
        .and_then(|prefix| prefix.checked_add(third_shifted))
        .and_then(|prefix| prefix.checked_add(fourth))
        .ok_or(super::types::DiagnosticCodeParseError::InvalidFormat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbolic_to_numeric_roundtrip() {
        for entry in CODE_REGISTRY {
            assert_eq!(
                symbolic_to_numeric(entry.symbolic),
                Some(entry.numeric),
                "symbolic → numeric failed for {}",
                entry.symbolic
            );
            assert_eq!(
                numeric_to_symbolic(entry.numeric),
                Some(entry.symbolic),
                "numeric → symbolic failed for 0x{:04X}",
                entry.numeric
            );
        }
    }

    #[test]
    fn category_from_numeric_matches_registry() {
        for entry in CODE_REGISTRY {
            assert_eq!(
                category_from_numeric(entry.numeric),
                entry.category,
                "category mismatch for {} (0x{:04X})",
                entry.symbolic,
                entry.numeric
            );
        }
    }

    #[test]
    fn is_supported_code_matches_registry() {
        for entry in CODE_REGISTRY {
            assert!(
                is_supported_code(entry.numeric),
                "is_supported_code should return true for registry entry 0x{:04X}",
                entry.numeric
            );
        }
        // Codes not in the registry must return false.
        assert!(!is_supported_code(0x0000));
        assert!(!is_supported_code(0xFFFF));
        assert!(!is_supported_code(0x07FF)); // gap between ContractDiscovery and Compilation
    }
}
