//! YAML text envelope emission.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
use saphyr::{Mapping, Scalar, Yaml, YamlEmitter};

use crate::emitter::error::EmitterError;
use crate::envelope::{DiagnosticEntry, OutputEnvelope};

pub const TEXT_SCHEMA_VERSION: &str = "velvet-ballastics/cli-output/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlEnvelope {
    pub schema_version: String,
    pub kind: String,
    pub command: String,
    pub exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Vec<DiagnosticEntry>>,
}

impl YamlEnvelope {
    pub fn from_envelope(envelope: &OutputEnvelope, exit_code: u8) -> Self {
        Self {
            schema_version: TEXT_SCHEMA_VERSION.to_string(),
            kind: envelope.kind.name().to_string(),
            command: envelope.metadata.command.clone(),
            exit_code,
            data: envelope.data.as_ref().map(|p| p.as_json().clone()),
            diagnostics: None,
        }
    }
}

#[cfg(feature = "std")]
pub fn encode_yaml<T: Serialize>(payload: &T) -> Result<String, EmitterError> {
    let json_value = serde_json::to_value(payload).map_err(|_| EmitterError::YamlEncodeFailed)?;
    let mut output = String::new();
    let mut emitter = YamlEmitter::new(&mut output);

    let doc = json_value_to_yaml(&json_value)?;
    emitter
        .dump(&doc)
        .map_err(|_| EmitterError::YamlEncodeFailed)?;
    Ok(output)
}

#[cfg(feature = "std")]
fn json_value_to_yaml(value: &serde_json::Value) -> Result<Yaml<'static>, EmitterError> {
    use alloc::borrow::Cow;
    match value {
        serde_json::Value::Null => Ok(Yaml::Value(Scalar::Null)),
        serde_json::Value::Bool(b) => Ok(Yaml::Value(Scalar::Boolean(*b))),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Yaml::Value(Scalar::Integer(i)))
            } else if let Some(u) = n.as_u64() {
                let val = i64::try_from(u).unwrap_or(i64::MAX);
                Ok(Yaml::Value(Scalar::Integer(val)))
            } else if let Some(f) = n.as_f64() {
                Ok(Yaml::Value(Scalar::String(Cow::Owned(f.to_string()))))
            } else {
                Ok(Yaml::Value(Scalar::Null))
            }
        }
        serde_json::Value::String(s) => Ok(Yaml::Value(Scalar::String(Cow::Owned(s.clone())))),
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_value_to_yaml).collect();
            items.map(Yaml::Sequence)
        }
        serde_json::Value::Object(obj) => {
            let mut mapping = Mapping::new();
            for (k, v) in obj {
                let key = Yaml::Value(Scalar::String(Cow::Owned(k.clone())));
                let val = json_value_to_yaml(v)?;
                mapping.insert(key, val);
            }
            Ok(Yaml::Mapping(mapping))
        }
    }
}

#[allow(dead_code)]
pub(crate) fn validate_no_ansi(text: &str) -> Result<(), EmitterError> {
    if text.contains('\x1B') {
        return Err(EmitterError::AnsiForbidden);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emitter::error::EmitterError;
    use crate::envelope::{
        EnvelopeKind, MetadataEnvelope, OutputEnvelope, PayloadEnvelope, SchemaVersion,
    };
    use vb_core::ids::RunId;

    #[test]
    fn validate_no_ansi_accepts_plain_text() {
        assert!(validate_no_ansi("hello world").is_ok());
        assert!(validate_no_ansi("").is_ok());
        assert!(validate_no_ansi("line1\nline2").is_ok());
    }

    #[test]
    fn validate_no_ansi_rejects_ansi() {
        assert!(matches!(
            validate_no_ansi("\x1B[31mred text\x1B[0m"),
            Err(EmitterError::AnsiForbidden)
        ));
        assert!(matches!(
            validate_no_ansi("\x1B[1;2;3m"),
            Err(EmitterError::AnsiForbidden)
        ));
    }

    #[cfg(feature = "std")]
    #[test]
    fn yaml_envelope_from_envelope() {
        let run_id = RunId::new(123);
        let metadata = MetadataEnvelope::new(run_id, "verify".to_string(), 1000);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"passed": true}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Success,
            metadata,
            Some(payload),
            Vec::new(),
        )
        .expect("envelope build should succeed");

        let yaml_env = YamlEnvelope::from_envelope(&envelope, 0);

        assert_eq!(yaml_env.schema_version, TEXT_SCHEMA_VERSION);
        assert_eq!(yaml_env.kind, "Success");
        assert_eq!(yaml_env.command, "verify");
        assert_eq!(yaml_env.exit_code, 0);
        assert!(yaml_env.data.is_some());
        assert!(yaml_env.diagnostics.is_none());
    }

    #[cfg(feature = "std")]
    #[test]
    fn yaml_envelope_from_envelope_error_kind() {
        let run_id = RunId::new(456);
        let metadata = MetadataEnvelope::new(run_id, "run".to_string(), 2000);
        let payload = PayloadEnvelope::from_json(serde_json::json!({"error": "failed"}));
        let envelope = OutputEnvelope::new(
            SchemaVersion::CURRENT,
            EnvelopeKind::Error,
            metadata,
            Some(payload),
            Vec::new(),
        )
        .expect("envelope build should succeed");

        let yaml_env = YamlEnvelope::from_envelope(&envelope, 1);

        assert_eq!(yaml_env.kind, "Error");
        assert_eq!(yaml_env.exit_code, 1);
    }

    #[cfg(feature = "std")]
    #[test]
    fn yaml_envelope_from_envelope_diagnostic_report_kind() {
        let run_id = RunId::new(789);
        let metadata = MetadataEnvelope::new(run_id, "diag".to_string(), 3000);
        let diagnostics =
            vec![DiagnosticEntry::new("VB001".to_string(), "msg".to_string(), None).unwrap()];
        let envelope =
            OutputEnvelope::diagnostic_report(SchemaVersion::CURRENT, metadata, diagnostics)
                .expect("envelope build should succeed");

        let yaml_env = YamlEnvelope::from_envelope(&envelope, 0);

        assert_eq!(yaml_env.kind, "DiagnosticReport");
        assert!(yaml_env.diagnostics.is_none()); // Diagnostics are not serialized in YamlEnvelope
    }

    // =====================================================================
    // json_value_to_yaml — all JSON value types
    // =====================================================================

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_null() {
        use saphyr::{Scalar, Yaml};
        let result = json_value_to_yaml(&serde_json::json!(null)).unwrap();
        assert!(matches!(result, Yaml::Value(Scalar::Null)));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_bool() {
        use saphyr::{Scalar, Yaml};
        let result_true = json_value_to_yaml(&serde_json::json!(true)).unwrap();
        assert!(matches!(result_true, Yaml::Value(Scalar::Boolean(true))));

        let result_false = json_value_to_yaml(&serde_json::json!(false)).unwrap();
        assert!(matches!(result_false, Yaml::Value(Scalar::Boolean(false))));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_integer() {
        use saphyr::{Scalar, Yaml};
        let result = json_value_to_yaml(&serde_json::json!(42)).unwrap();
        assert!(matches!(result, Yaml::Value(Scalar::Integer(42))));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_negative_integer() {
        use saphyr::{Scalar, Yaml};
        let result = json_value_to_yaml(&serde_json::json!(-17)).unwrap();
        assert!(matches!(result, Yaml::Value(Scalar::Integer(-17))));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_u64_converts_to_i64() {
        use saphyr::{Scalar, Yaml};
        // u64 that fits in i64
        let result = json_value_to_yaml(&serde_json::json!(100u64)).unwrap();
        assert!(matches!(result, Yaml::Value(Scalar::Integer(100))));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_f64_converts_to_string() {
        use saphyr::{Scalar, Yaml};
        let result = json_value_to_yaml(&serde_json::json!(3.14)).unwrap();
        assert!(matches!(result, Yaml::Value(Scalar::String(_))));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_string() {
        use saphyr::{Scalar, Yaml};
        let result = json_value_to_yaml(&serde_json::json!("hello world")).unwrap();
        assert!(matches!(result, Yaml::Value(Scalar::String(ref s)) if s.contains("hello")));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_array() {
        use saphyr::Yaml;
        let result = json_value_to_yaml(&serde_json::json!([1, "two", null])).unwrap();
        assert!(matches!(result, Yaml::Sequence(_)));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_object() {
        use saphyr::Yaml;
        let result = json_value_to_yaml(&serde_json::json!({"key": "value"})).unwrap();
        assert!(matches!(result, Yaml::Mapping(_)));
    }

    #[cfg(feature = "std")]
    #[test]
    fn json_value_to_yaml_nested() {
        use saphyr::Yaml;
        let json = serde_json::json!({
            "outer": {
                "inner": [1, 2, 3]
            }
        });
        let result = json_value_to_yaml(&json).unwrap();
        assert!(matches!(result, Yaml::Mapping(_)));
    }

    // =====================================================================
    // TEXT_SCHEMA_VERSION constant
    // =====================================================================

    #[test]
    fn text_schema_version_value() {
        assert_eq!(TEXT_SCHEMA_VERSION, "velvet-ballastics/cli-output/v1");
    }

    // =====================================================================
    // YamlEnvelope serde roundtrip
    // =====================================================================

    #[cfg(feature = "std")]
    #[test]
    fn yaml_envelope_serde() {
        let ye = YamlEnvelope {
            schema_version: TEXT_SCHEMA_VERSION.to_string(),
            kind: "Success".to_string(),
            command: "test".to_string(),
            exit_code: 0,
            data: Some(serde_json::json!({"x": 1})),
            diagnostics: None,
        };
        let json = serde_json::to_string(&ye).unwrap();
        assert!(json.contains("Success"));
        assert!(json.contains("test"));
        let deserialized: YamlEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, "Success");
        assert_eq!(deserialized.command, "test");
    }
}
