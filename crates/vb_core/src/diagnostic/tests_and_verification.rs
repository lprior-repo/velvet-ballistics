// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::diagnostic::{
        CodeCategory, Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity, SymbolicCode,
        CODE_REGISTRY, is_supported_code, numeric_to_symbolic, symbolic_to_numeric,
    };
    use crate::span::Span;
    use core::str::FromStr;

    // ---- DiagnosticCode existing tests ----

    #[test]
    fn diagnostic_code_preserves_packed_value() {
        let code = DiagnosticCode::new(0x0101);

        assert_eq!(code.code(), 0x0101);
        assert_eq!(code.to_string(), "E0101");
    }

    #[test]
    fn diagnostic_code_parses_supported_ranges() {
        assert_eq!(
            DiagnosticCode::from_str("E0101"),
            Ok(DiagnosticCode::new(0x0101))
        );
        assert_eq!(
            DiagnosticCode::from_str("E010B"),
            Ok(DiagnosticCode::new(0x010B))
        );
        assert_eq!(
            DiagnosticCode::from_str("E0409"),
            Ok(DiagnosticCode::new(0x0409))
        );
        assert_eq!(
            DiagnosticCode::from_str("E040C"),
            Ok(DiagnosticCode::new(0x040C))
        );
        assert_eq!(
            DiagnosticCode::from_str("E1315"),
            Ok(DiagnosticCode::new(0x1315))
        );
        assert_eq!(
            DiagnosticCode::from_str("E4015"),
            Ok(DiagnosticCode::new(0x4015))
        );
        // New: E3020 action/audit codes (REPAIR-7 range fix)
        assert_eq!(
            DiagnosticCode::from_str("E3020"),
            Ok(DiagnosticCode::new(0x3020))
        );
        // New: E05xx gate verifier codes
        assert_eq!(
            DiagnosticCode::from_str("E0501"),
            Ok(DiagnosticCode::new(0x0501))
        );
        // New: E06xx contract discovery codes
        assert_eq!(
            DiagnosticCode::from_str("E0601"),
            Ok(DiagnosticCode::new(0x0601))
        );
        // New: E4020 boundary
        assert_eq!(
            DiagnosticCode::from_str("E4020"),
            Ok(DiagnosticCode::new(0x4020))
        );
    }

    #[test]
    fn diagnostic_code_rejects_malformed_or_unsupported_input() {
        assert_eq!(
            DiagnosticCode::from_str("0101"),
            Err(DiagnosticCodeParseError::InvalidFormat)
        );
        assert_eq!(
            DiagnosticCode::from_str("E010C"),
            Err(DiagnosticCodeParseError::UnsupportedCode)
        );
        assert_eq!(
            DiagnosticCode::from_str("E0410"),
            Err(DiagnosticCodeParseError::UnsupportedCode)
        );
    }

    // ---- SymbolicCode tests ----

    #[test]
    fn symbolic_code_from_static_known_code() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY");
        assert!(code.is_some());
        assert_eq!(code.expect("should be Some").as_str(), "DUPLICATE_KEY");
    }

    #[test]
    fn symbolic_code_from_static_unknown_code() {
        let code = SymbolicCode::from_static("BOGUS_CODE");
        assert!(code.is_none());
    }

    #[test]
    fn symbolic_code_numeric_code_roundtrip() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        assert_eq!(code.numeric_code(), Some(0x0101));
        assert_eq!(code.as_diagnostic_code(), Some(DiagnosticCode::new(0x0101)));
    }

    #[test]
    fn symbolic_code_display_is_name_not_hex() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        assert_eq!(code.to_string(), "DUPLICATE_KEY");
    }

    #[test]
    fn symbolic_code_is_copy() {
        let a = SymbolicCode::from_static("TYPE_MISMATCH").unwrap();
        let b = a;
        assert_eq!(a, b);
        // Both usable after copy
        assert_eq!(a.as_str(), "TYPE_MISMATCH");
        assert_eq!(b.as_str(), "TYPE_MISMATCH");
    }

    #[test]
    fn symbolic_code_category() {
        let schema = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        assert_eq!(schema.category(), Some(CodeCategory::Schema));

        let gate = SymbolicCode::from_static("EXPRESSION_STACK_EXCEEDED").unwrap();
        assert_eq!(gate.category(), Some(CodeCategory::Gate));

        let runtime = SymbolicCode::from_static("RUNTIME_TIMEOUT").unwrap();
        assert_eq!(runtime.category(), Some(CodeCategory::Runtime));
    }

    #[test]
    fn symbolic_code_from_str_accepts_registered_name() {
        let result: Result<SymbolicCode, _> = "DUPLICATE_KEY".parse();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_str(), "DUPLICATE_KEY");
    }

    #[test]
    fn symbolic_code_from_str_rejects_unknown_name() {
        let result: Result<SymbolicCode, _> = "BOGUS_CODE".parse();
        assert!(result.is_err());
    }

    // ---- CODE_REGISTRY tests ----

    #[test]
    fn registry_symbolic_to_numeric_roundtrip() {
        let numeric = symbolic_to_numeric("DUPLICATE_KEY");
        assert_eq!(numeric, Some(0x0101));

        let symbolic = numeric_to_symbolic(0x0101);
        assert_eq!(symbolic, Some("DUPLICATE_KEY"));
    }

    #[test]
    fn registry_all_codes_non_zero() {
        for entry in CODE_REGISTRY {
            assert_ne!(
                entry.numeric, 0,
                "code {} has zero numeric value",
                entry.symbolic
            );
        }
    }

    #[test]
    fn registry_no_duplicate_numeric() {
        for i in 0..CODE_REGISTRY.len() {
            for j in (i + 1)..CODE_REGISTRY.len() {
                assert_ne!(
                    CODE_REGISTRY[i].numeric, CODE_REGISTRY[j].numeric,
                    "duplicate numeric {:04X} for {} and {}",
                    CODE_REGISTRY[i].numeric, CODE_REGISTRY[i].symbolic, CODE_REGISTRY[j].symbolic,
                );
            }
        }
    }

    #[test]
    fn registry_no_duplicate_symbolic() {
        for i in 0..CODE_REGISTRY.len() {
            for j in (i + 1)..CODE_REGISTRY.len() {
                assert_ne!(
                    CODE_REGISTRY[i].symbolic, CODE_REGISTRY[j].symbolic,
                    "duplicate symbolic '{}' at indices {} and {}",
                    CODE_REGISTRY[i].symbolic, i, j,
                );
            }
        }
    }

    #[test]
    fn diagnostic_code_symbolic_lookup_known_code() {
        let dc = DiagnosticCode::new(0x0101);
        let sc = dc.symbolic_code();
        assert!(sc.is_some());
        assert_eq!(sc.unwrap().as_str(), "DUPLICATE_KEY");
    }

    #[test]
    fn diagnostic_code_symbolic_lookup_unsupported_code() {
        let dc = DiagnosticCode::new(0xDEAD);
        let sc = dc.symbolic_code();
        assert!(sc.is_none());
    }

    // ---- Serialization tests ----

    #[test]
    fn symbolic_code_serde_json_roundtrip() {
        let code =
            SymbolicCode::from_static("DUPLICATE_KEY").expect("DUPLICATE_KEY should be registered");
        let json =
            serde_json::to_string(&code).expect("serialization must succeed for SymbolicCode");
        assert_eq!(json, "\"DUPLICATE_KEY\"");
        let restored: SymbolicCode =
            serde_json::from_str(&json).expect("deserialization must succeed for registered code");
        assert_eq!(restored, code);
    }

    #[test]
    fn symbolic_code_serde_json_rejects_unknown() {
        let result: Result<SymbolicCode, _> = serde_json::from_str("\"BOGUS_CODE\"");
        assert!(result.is_err(), "unregistered codes must be rejected");
    }

    // ---- Diagnostic tests ----

    #[test]
    fn diagnostic_new_from_symbolic_code() {
        let code = SymbolicCode::from_static("DUPLICATE_KEY").unwrap();
        let diag = Diagnostic::new(
            code,
            Box::<str>::from("duplicate key found"),
            Severity::Error,
            Span::ZERO,
            None,
        );

        assert_eq!(diag.code, code);
        assert_eq!(diag.numeric_code.code(), 0x0101);
        assert_eq!(diag.message.as_ref(), "duplicate key found");
        assert_eq!(diag.severity, Severity::Error);
        // Invariant: numeric_code.symbolic_code() == Some(code)
        assert_eq!(diag.numeric_code.symbolic_code(), Some(code));
    }

    #[test]
    fn diagnostic_from_numeric_when_registered() {
        let diag = Diagnostic::from_numeric(
            DiagnosticCode::new(0x0101),
            Box::<str>::from("duplicate key"),
            Severity::Error,
            Span::ZERO,
            None,
        );

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.code.as_str(), "DUPLICATE_KEY");
        assert_eq!(diag.numeric_code.code(), 0x0101);
    }

    #[test]
    fn diagnostic_from_numeric_when_unregistered() {
        let diag = Diagnostic::from_numeric(
            DiagnosticCode::new(0xDEAD),
            Box::<str>::from("unknown"),
            Severity::Error,
            Span::ZERO,
            None,
        );

        assert!(diag.is_none());
    }

    // ---- DiagnosticCodeParseError exact variant assertions ----

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_missing_prefix() {
        let result = DiagnosticCode::from_str("0101");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_hex_digits() {
        let result = DiagnosticCode::from_str("E010G");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_too_short() {
        let result = DiagnosticCode::from_str("E01");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_too_long() {
        let result = DiagnosticCode::from_str("E010101");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_invalid_format_when_empty() {
        let result = DiagnosticCode::from_str("");
        assert_eq!(result, Err(DiagnosticCodeParseError::InvalidFormat));
    }

    #[test]
    fn diagnostic_code_parse_error_unsupported_code_when_out_of_range() {
        let result = DiagnosticCode::from_str("E0410");
        assert_eq!(result, Err(DiagnosticCodeParseError::UnsupportedCode));
    }

    #[test]
    fn diagnostic_code_parse_error_unsupported_code_when_fully_outside_ranges() {
        let result = DiagnosticCode::from_str("E9999");
        assert_eq!(result, Err(DiagnosticCodeParseError::UnsupportedCode));
    }

    // ---- is_supported_code extended range tests ----

    #[test]
    fn is_supported_code_accepts_e0501() {
        assert!(is_supported_code(0x0501));
    }

    #[test]
    fn is_supported_code_accepts_e0601() {
        assert!(is_supported_code(0x0601));
    }

    #[test]
    fn is_supported_code_accepts_e4020() {
        assert!(is_supported_code(0x4020));
    }

    #[test]
    fn is_supported_code_accepts_e402e() {
        assert!(is_supported_code(0x402E));
    }

    #[test]
    fn is_supported_code_rejects_e0604() {
        assert!(!is_supported_code(0x0604));
    }

    // ---- is_supported_code REPAIR-7: action/audit codes (0x3020-0x3022) ----

    #[test]
    fn is_supported_code_accepts_e3020() {
        assert!(
            is_supported_code(0x3020),
            "ACTION_RESULT_AUDIT_MISMATCH"
        );
    }

    #[test]
    fn is_supported_code_accepts_e3021() {
        assert!(
            is_supported_code(0x3021),
            "ACTION_TYPE_CONSTRAINT_FAIL"
        );
    }

    #[test]
    fn is_supported_code_accepts_e3022() {
        assert!(
            is_supported_code(0x3022),
            "ACTION_CIRCUIT_BREAKER_OPEN"
        );
    }

    #[test]
    fn is_supported_code_rejects_e301c_through_e301f() {
        // 0x301C-0x301F are genuine gaps between Runtime 0x301B and
        // action/audit codes at 0x3020-0x3022.
        for code in 0x301Cu16..=0x301F {
            assert!(
                !is_supported_code(code),
                "E{:04X} must be rejected",
                code
            );
        }
    }
}
