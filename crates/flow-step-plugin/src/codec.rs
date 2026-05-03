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
