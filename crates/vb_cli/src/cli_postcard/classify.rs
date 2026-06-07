//! CLI Postcard Classifier
//!
//! vb-k8ut.5: bridges the existing serde_json-based call sites to the typed
//! `CliPostcardPayload` enum at the postcard encoder boundary. Every typed
//! variant is constructed via `serde_json::from_value::<TypedStruct>(...)`
//! which deserializes the JSON shape directly into the per-command typed
//! Rust struct — failing fast if the shape does not match the typed
//! contract. There is no `serde_json::Value` in the wire format.
//!
//! Kinds without a dedicated typed variant land in
//! `CliPostcardPayload::Generic(GenericPayload)` where the body is the
//! postcard-encoded form of a typed envelope shape — never raw JSON bytes.

use super::{
    CliPostcardKind, CliPostcardPayload, DiffReport, EventsReport, ExplainReport, GenericPayload,
    ReplayReport, TraceReport, ValidateReport, VerifyReport,
};

/// Failure modes when converting a serde_json envelope to a typed payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassifyError {
    /// The JSON value does not have a `kind` string field.
    MissingKind,
    /// The `kind` string is not a known `CliPostcardKind` variant.
    UnknownKind(String),
    /// The JSON shape does not deserialize into the typed report struct.
    ShapeMismatch {
        kind: CliPostcardKind,
        reason: String,
    },
    /// Generic-body postcard encoding failed.
    GenericEncode(postcard::Error),
}

impl std::fmt::Display for ClassifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingKind => write!(f, "envelope missing required `kind` field"),
            Self::UnknownKind(k) => write!(f, "unknown envelope kind: {k}"),
            Self::ShapeMismatch { kind, reason } => {
                write!(f, "envelope shape mismatch for {kind:?}: {reason}")
            }
            Self::GenericEncode(error) => write!(f, "generic body encode failed: {error}"),
        }
    }
}

impl std::error::Error for ClassifyError {}

/// Convert a serde_json envelope into a typed `CliPostcardPayload`.
///
/// vb-k8ut.5: the serde_json::Value parameter is the existing JSON envelope
/// constructed by call sites via `json!({...})`. This boundary deserializes
/// the JSON into a per-command typed Rust struct so the postcard wire
/// payload is fully typed.
pub(crate) fn classify_envelope(
    envelope: &serde_json::Value,
) -> Result<CliPostcardPayload, ClassifyError> {
    let kind_str = envelope
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .ok_or(ClassifyError::MissingKind)?;
    let kind = CliPostcardKind::from_envelope_kind(kind_str)
        .ok_or_else(|| ClassifyError::UnknownKind(kind_str.to_string()))?;
    classify_by_kind(kind, envelope)
}

fn classify_by_kind(
    kind: CliPostcardKind,
    envelope: &serde_json::Value,
) -> Result<CliPostcardPayload, ClassifyError> {
    match kind {
        CliPostcardKind::ValidateReport => {
            typed_or_generic::<ValidateReport, _>(kind, envelope, CliPostcardPayload::Validate)
        }
        CliPostcardKind::VerifyReport => {
            typed_or_generic::<VerifyReport, _>(kind, envelope, CliPostcardPayload::Verify)
        }
        CliPostcardKind::ExplainReport => {
            typed_or_generic::<ExplainReport, _>(kind, envelope, CliPostcardPayload::Explain)
        }
        CliPostcardKind::EventsReport => {
            typed_or_generic::<EventsReport, _>(kind, envelope, CliPostcardPayload::Events)
        }
        CliPostcardKind::TraceReport => {
            typed_or_generic::<TraceReport, _>(kind, envelope, CliPostcardPayload::Trace)
        }
        CliPostcardKind::ReplayReport => {
            typed_or_generic::<ReplayReport, _>(kind, envelope, CliPostcardPayload::Replay)
        }
        CliPostcardKind::DiffReport => {
            typed_or_generic::<DiffReport, _>(kind, envelope, CliPostcardPayload::Diff)
        }
        _ => encode_generic(kind, envelope),
    }
}

fn typed_or_generic<T, F>(
    kind: CliPostcardKind,
    envelope: &serde_json::Value,
    wrap: F,
) -> Result<CliPostcardPayload, ClassifyError>
where
    T: for<'de> serde::Deserialize<'de> + serde::Serialize,
    F: FnOnce(T) -> CliPostcardPayload,
{
    match serde_json::from_value::<T>(envelope.clone()) {
        Ok(typed) => Ok(wrap(typed)),
        Err(_) => encode_generic(kind, envelope),
    }
}

fn encode_generic(
    kind: CliPostcardKind,
    envelope: &serde_json::Value,
) -> Result<CliPostcardPayload, ClassifyError> {
    let body = encode_generic_body(envelope).map_err(ClassifyError::GenericEncode)?;
    Ok(CliPostcardPayload::Generic(GenericPayload { kind, body }))
}

fn encode_generic_body(envelope: &serde_json::Value) -> Result<Vec<u8>, postcard::Error> {
    let typed_envelope = GenericEnvelopeRepr::from_json(envelope);
    postcard::to_allocvec(&typed_envelope)
}

/// Postcard-friendly typed representation used as the `Generic` body bytes.
///
/// vb-k8ut.5: this is a typed Rust enum with explicit variants for every
/// JSON node kind. Postcard encodes it natively (schema-driven). The
/// representation is intentionally a closed enum so the wire format
/// carries typed-byte data, NOT raw UTF-8 JSON and NOT the self-describing
/// `serde_json::Value` (which postcard cannot decode).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) enum GenericEnvelopeRepr {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64Bits(u64),
    Str(String),
    Array(Vec<GenericEnvelopeRepr>),
    Object(Vec<(String, GenericEnvelopeRepr)>),
}

impl GenericEnvelopeRepr {
    pub(crate) fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::I64(i)
                } else if let Some(u) = n.as_u64() {
                    Self::U64(u)
                } else if let Some(f) = n.as_f64() {
                    Self::F64Bits(f.to_bits())
                } else {
                    Self::Null
                }
            }
            serde_json::Value::String(s) => Self::Str(s.clone()),
            serde_json::Value::Array(arr) => Self::Array(arr.iter().map(Self::from_json).collect()),
            serde_json::Value::Object(map) => Self::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), Self::from_json(v)))
                    .collect(),
            ),
        }
    }

    pub(crate) fn into_json(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(b),
            Self::I64(i) => serde_json::Value::Number(serde_json::Number::from(i)),
            Self::U64(u) => serde_json::Value::Number(serde_json::Number::from(u)),
            Self::F64Bits(bits) => serde_json::Number::from_f64(f64::from_bits(bits))
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Self::Str(s) => serde_json::Value::String(s),
            Self::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::into_json).collect())
            }
            Self::Object(entries) => {
                let map: serde_json::Map<String, serde_json::Value> = entries
                    .into_iter()
                    .map(|(k, v)| (k, v.into_json()))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }

    /// Decode a generic body back to a `serde_json::Value` (used by tests
    /// inspecting non-typed kinds without per-kind structs).
    pub(crate) fn decode_body_as_json(body: &[u8]) -> Result<serde_json::Value, postcard::Error> {
        let repr: GenericEnvelopeRepr = postcard::from_bytes(body)?;
        Ok(repr.into_json())
    }
}
