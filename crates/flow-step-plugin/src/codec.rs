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
}
