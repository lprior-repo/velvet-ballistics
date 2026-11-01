//! Behavior tests proving harness↔production agreement for vb-94w9z.
//!
//! The kani harnesses under `crates/vb_core/src/kani/` now execute the
//! production diagnostic types directly. These runtime tests prove the
//! same properties over the full production `CODE_REGISTRY` so that
//! non-kani CI guards them too:
//!
//! - `SymbolicCode::from_static` / `FromStr` accept exactly the
//!   registered symbolic names.
//! - `DiagnosticCode::from_str` succeeds iff the code is registered
//!   (full `u16` sweep against `is_registered_numeric`).
//! - symbolic/numeric lookups are a bijection over the registry.
//! - `category_from_numeric` and `DiagnosticCode::category` agree
//!   with each registry entry's category.
//! - `Diagnostic::new` preserves the documented
//!   `numeric_code.symbolic_code() == Some(code)` invariant.
//! - `SymbolicCode` serde round-trips through `serde_json`.

use core::str::FromStr as _;

use vb_core::diagnostic::{
    CODE_REGISTRY, Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity, SymbolicCode,
    category_from_numeric, is_registered_numeric, numeric_to_symbolic, symbolic_to_numeric,
};
use vb_core::span::Span;

mod from_static_accepts_registry {
    use super::*;

    #[test]
    fn every_registered_symbolic_constructs() {
        for entry in CODE_REGISTRY {
            let code = SymbolicCode::from_static(entry.symbolic);
            assert!(code.is_some(), "registered {}", entry.symbolic);
            if let Some(code) = code {
                assert_eq!(code.as_str(), entry.symbolic);
                assert_eq!(code.numeric_code(), Some(entry.numeric));
                assert_eq!(code.category(), Some(entry.category));
            }
        }
    }

    #[test]
    fn rejects_unregistered_strings() {
        for unknown in [
            "__DEFINITELY_NOT_REGISTERED__",
            "",
            "duplicate_key",
            "E0101",
        ] {
            assert!(
                SymbolicCode::from_static(unknown).is_none(),
                "must reject {unknown:?}"
            );
            assert!(
                SymbolicCode::from_str(unknown).is_err(),
                "FromStr must reject {unknown:?}"
            );
        }
    }
}

mod symbolic_from_str_registry {
    use super::*;

    #[test]
    fn every_registered_symbolic_parses() {
        for entry in CODE_REGISTRY {
            let parsed = SymbolicCode::from_str(entry.symbolic);
            assert!(parsed.is_ok(), "registered {}", entry.symbolic);
            if let Ok(code) = parsed {
                assert_eq!(code.as_str(), entry.symbolic);
            }
        }
    }
}

mod numeric_from_str_agreement {
    use super::*;

    #[test]
    fn every_registered_numeric_parses_and_round_trips() {
        for entry in CODE_REGISTRY {
            let text = format!("E{:04X}", entry.numeric);
            let parsed = DiagnosticCode::from_str(&text);
            assert!(parsed.is_ok(), "registered {text}");
            if let Ok(code) = parsed {
                assert_eq!(code.code(), entry.numeric);
            }
        }
    }

    #[test]
    fn from_str_succeeds_iff_registered_over_all_u16() {
        for raw in 0..=u16::MAX {
            let text = format!("E{raw:04X}");
            let result = DiagnosticCode::from_str(&text);
            if is_registered_numeric(raw) {
                assert_eq!(result.map(|code| code.code()), Ok(raw));
            } else {
                assert_eq!(result, Err(DiagnosticCodeParseError::UnsupportedCode));
            }
        }
    }

    #[test]
    fn malformed_inputs_are_invalid_format() {
        for bad in ["", "X0101", "E101", "E01011", "E01ZZ", "e0101"] {
            assert_eq!(
                DiagnosticCode::from_str(bad),
                Err(DiagnosticCodeParseError::InvalidFormat),
                "malformed {bad:?}"
            );
        }
    }
}

mod registry_bijection {
    use super::*;

    #[test]
    fn symbolic_numeric_lookups_are_inverse() {
        for entry in CODE_REGISTRY {
            assert_eq!(symbolic_to_numeric(entry.symbolic), Some(entry.numeric));
            assert_eq!(numeric_to_symbolic(entry.numeric), Some(entry.symbolic));
        }
    }

    #[test]
    fn no_zero_code_and_no_duplicates() {
        let mut seen_numeric = [false; 65536];
        for entry in CODE_REGISTRY {
            assert_ne!(entry.numeric, 0, "zero code is reserved");
            let slot: usize = usize::from(entry.numeric);
            assert!(!seen_numeric[slot], "duplicate {:04X}", entry.numeric);
            seen_numeric[slot] = true;
        }
    }
}

mod category_agreement {
    use super::*;

    #[test]
    fn registry_category_matches_classification() {
        for entry in CODE_REGISTRY {
            assert_eq!(category_from_numeric(entry.numeric), entry.category);
            assert_eq!(
                DiagnosticCode::new(entry.numeric).category(),
                Some(entry.category)
            );
        }
    }
}

mod diagnostic_constructor_invariant {
    use super::*;

    #[test]
    fn new_derives_matching_numeric_code() {
        for entry in CODE_REGISTRY {
            let sym = SymbolicCode::from_static(entry.symbolic);
            assert!(
                sym.is_some(),
                "registered {} must construct",
                entry.symbolic
            );
            if let Some(sym) = sym {
                let diagnostic = Diagnostic::new(
                    sym,
                    "unification probe".into(),
                    Severity::Error,
                    Span::ZERO,
                    None,
                );
                assert_eq!(diagnostic.code, sym);
                assert_eq!(diagnostic.numeric_code.code(), entry.numeric);
                assert_eq!(diagnostic.numeric_code.symbolic_code(), Some(sym));
            }
        }
    }
}

mod serde_round_trip {
    use super::*;

    #[test]
    fn symbolic_code_json_round_trip() {
        for entry in CODE_REGISTRY {
            let code = SymbolicCode::from_static(entry.symbolic);
            assert!(
                code.is_some(),
                "registered {} must construct",
                entry.symbolic
            );
            if let Some(code) = code {
                let json = serde_json::to_string(&code);
                assert!(json.is_ok(), "serialize {}", entry.symbolic);
                if let Ok(json) = json {
                    let back: Result<SymbolicCode, _> = serde_json::from_str(&json);
                    assert!(back.is_ok(), "deserialize {}", entry.symbolic);
                    if let Ok(code) = back {
                        assert_eq!(code.as_str(), entry.symbolic);
                    }
                }
            }
        }
    }

    #[test]
    fn deserialize_rejects_unknown() {
        let back: Result<SymbolicCode, _> = serde_json::from_str("\"NOT_A_CODE\"");
        assert!(back.is_err(), "unknown code must be rejected");
    }
}
