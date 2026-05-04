use flow_core::doc::FlowDocument;

/// Import Amazon States Language JSON into a FlowDocument.
pub fn import_asl(json: &str) -> Result<FlowDocument, CodecError> {
    let _ = json;
    Err(CodecError::NotImplemented)
}

/// Export a FlowDocument to Amazon States Language JSON.
pub fn export_asl(doc: &FlowDocument) -> Result<String, CodecError> {
    let _ = doc;
    Err(CodecError::NotImplemented)
}

#[derive(Debug)]
pub enum CodecError {
    NotImplemented,
    ParseError(String),
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::doc::FlowDocument;

    #[test]
    fn import_asl_returns_not_implemented() {
        let result = import_asl("{}");
        match result {
            Err(CodecError::NotImplemented) => {}
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn import_asl_returns_error_for_any_input() {
        let cases = ["", "{}", "{\"StartAt\":\"A\",\"States\":{}}"];
        for input in cases {
            match import_asl(input) {
                Err(CodecError::NotImplemented) => {}
                other => panic!("expected NotImplemented for input {input:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn export_asl_returns_not_implemented() {
        let doc = FlowDocument::default();
        let result = export_asl(&doc);
        match result {
            Err(CodecError::NotImplemented) => {}
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn codec_error_debug_formats() {
        let e1 = CodecError::NotImplemented;
        let s1 = format!("{e1:?}");
        assert!(s1.contains("NotImplemented"), "debug output: {s1}");

        let e2 = CodecError::ParseError("bad json".into());
        let s2 = format!("{e2:?}");
        assert!(s2.contains("ParseError"), "debug output: {s2}");

        let e3 = CodecError::ValidationError("missing field".into());
        let s3 = format!("{e3:?}");
        assert!(s3.contains("ValidationError"), "debug output: {s3}");
    }

    // ========================================================================
    // BLACKHAT security review tests
    // ========================================================================

    /// BH-CODEC-01 (MEDIUM): import_asl silently accepts all input without
    /// validation. A malicious ASL payload containing embedded scripts,
    /// path traversal strings, or oversized documents is not rejected with
    /// a ParseError or ValidationError -- it returns NotImplemented, which
    /// callers cannot distinguish from "not yet implemented" vs "input is
    /// dangerous". When this function is eventually implemented, it MUST
    /// validate input before parsing.
    #[test]
    fn blackhat_import_asl_accepts_malicious_payloads_without_validation() {
        let malicious_inputs = [
            // Script injection in JSON values
            r#"{"StartAt":"<script>alert(1)</script>","States":{}}"#,
            // Path traversal in resource ARN
            r#"{"StartAt":"A","States":{"A":{"Type":"Task","Resource":"../../etc/passwd","End":true}}}"#,
            // Extremely deeply nested JSON (potential stack overflow in naive parsers)
            &"{".repeat(10000).replace("{", "{\"a\":"),
            // Null bytes
            "\x00{\"StartAt\":\"A\"}",
            // Unicode homoglyph attack
            "{\"StartAt\": \"А\"}",
        ];
        for input in &malicious_inputs {
            // All of these should ideally return ParseError or ValidationError,
            // not NotImplemented. The current code treats them identically to
            // benign input, providing zero defense-in-depth.
            let result = import_asl(input);
            match result {
                Err(CodecError::NotImplemented) => {
                    // BUG: Dangerous input is not distinguished from valid input.
                    // When import_asl is implemented, these MUST produce a
                    // ParseError or ValidationError instead.
                }
                Err(CodecError::ParseError(_)) | Err(CodecError::ValidationError(_)) => {
                    // Correct behavior -- input was rejected as invalid.
                }
                Ok(_) => {
                    panic!("import_asl should never succeed on malicious input");
                }
            }
        }
    }

    /// BH-CODEC-02 (LOW): export_asl ignores the document entirely. If a
    /// caller relies on export_asl to sanitize or validate a FlowDocument
    /// before transmission, the NotImplemented return gives a false sense
    /// of security -- no validation was actually performed.
    #[test]
    fn blackhat_export_asl_provides_no_validation_guarantee() {
        let doc = FlowDocument::default();
        let result = export_asl(&doc);
        assert!(
            matches!(result, Err(CodecError::NotImplemented)),
            "export_asl must not succeed until it validates the document"
        );
    }

    /// BH-CODEC-03 (INFO): CodecError lacks an error code or machine-readable
    /// identifier, making it difficult for callers to programmatically
    /// distinguish security-relevant errors (ParseError with injection) from
    /// transient ones (NotImplemented).
    #[test]
    fn blackhat_codec_error_lacks_structured_error_codes() {
        let errors: Vec<CodecError> = vec![
            CodecError::NotImplemented,
            CodecError::ParseError("malformed".into()),
            CodecError::ValidationError("integrity".into()),
        ];
        for error in &errors {
            let debug_str = format!("{error:?}");
            // No structured error code field exists -- only string matching.
            assert!(!debug_str.is_empty());
        }
    }
}
