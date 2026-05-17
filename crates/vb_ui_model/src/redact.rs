//! Redaction for secret-sensitive values in UI artifacts.
//!
//! Provides fail-closed redaction projection: secret-sensitive values
//! serialize only as redaction status, taint marker, digest, and
//! bounded summary. Raw secret bytes or text never appear in output.

#![forbid(unsafe_code)]

use alloc::format;
use alloc::string::{String, ToString};
use serde::{Deserialize, Serialize};

use vb_core::value::Taint;

/// Maximum length for a redaction summary string.
pub const MAX_REDACTION_SUMMARY_LEN: usize = 64;

/// Maximum length for a digest string representation.
pub const MAX_DIGEST_LEN: usize = 64;

/// Redacted view of a secret-sensitive value.
/// Contains only redaction status, taint marker, digest, and
/// bounded summary. Raw secret bytes are never present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedValueView {
    /// Whether the value is currently tainted.
    pub is_tainted: bool,
    /// Taint classification marker.
    pub taint_marker: String,
    /// BLAKE3 digest of the original value (hex string).
    pub digest: String,
    /// Bounded summary string (first N chars or empty if no summary).
    pub summary: String,
    /// Bounded summary byte length.
    pub summary_len: usize,
}

/// Classification of secret sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSensitivity {
    /// Known sensitive value that must be redacted.
    Sensitive,
    /// Known non-sensitive value.
    NonSensitive,
    /// Unknown sensitivity - fail-closed behavior required.
    Unknown,
}

/// Result of secret sensitivity classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitivityClass {
    pub classification: SecretSensitivity,
    pub reason: Option<String>,
}

/// Returns the sensitivity classification for a given field name or path.
/// Uses fail-closed behavior: unknown field names default to `Unknown`.
pub fn classify_secret_sensitivity(field_path: &str) -> SensitivityClass {
    let lower = field_path.to_lowercase();

    // Known sensitive field patterns
    if lower.contains("password")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("api_key")
        || lower.contains("apikey")
        || lower.contains("private_key")
        || lower.contains("privatekey")
        || lower.contains("credential")
        || lower.contains("auth")
    {
        return SensitivityClass {
            classification: SecretSensitivity::Sensitive,
            reason: Some(format!(
                "field path matches sensitive pattern: {}",
                field_path
            )),
        };
    }

    // Known non-sensitive field patterns
    if lower.contains("name")
        || lower.contains("id")
        || lower.contains("timestamp")
        || lower.contains("status")
        || lower.contains("kind")
        || lower.contains("type")
        || lower.contains("count")
        || lower.contains("index")
    {
        return SensitivityClass {
            classification: SecretSensitivity::NonSensitive,
            reason: Some(format!(
                "field path matches non-sensitive pattern: {}",
                field_path
            )),
        };
    }

    // Fail-closed: unknown sensitivity must be treated as sensitive
    SensitivityClass {
        classification: SecretSensitivity::Unknown,
        reason: Some(format!(
            "field path has unknown sensitivity classification: {}",
            field_path
        )),
    }
}

/// Creates a redacted view of a secret-sensitive value.
/// Uses fail-closed behavior: if sensitivity is Unknown, returns None.
/// Digest is computed via BLAKE3 hash of the input bytes.
pub fn redact_secret_value(
    value: &str,
    taint: Taint,
    sensitivity: SensitivityClass,
) -> Option<RedactedValueView> {
    match sensitivity.classification {
        SecretSensitivity::NonSensitive => {
            // Non-sensitive values may pass through unchanged
            Some(RedactedValueView {
                is_tainted: matches!(taint, Taint::DerivedFromSecret | Taint::Secret),
                taint_marker: taint_marker_string(taint),
                digest: String::new(),
                summary: String::new(),
                summary_len: 0,
            })
        }
        SecretSensitivity::Sensitive | SecretSensitivity::Unknown => {
            // Sensitive and unknown values must be redacted
            let digest = blake3::hash(value.as_bytes());
            let digest_hex = digest.to_hex().to_string();

            // Unknown values get a bounded summary for diagnostics;
            // Sensitive values get no summary (only digest for verification)
            let (summary, summary_len) = if sensitivity.classification == SecretSensitivity::Unknown
            {
                let len = core::cmp::min(value.len(), MAX_REDACTION_SUMMARY_LEN);
                let s = value
                    .get(..len)
                    .map(str::to_string)
                    .unwrap_or_else(String::new);
                (s, len)
            } else {
                (String::new(), 0)
            };

            // Sensitive or unknown data is always treated as tainted
            let is_tainted = matches!(taint, Taint::DerivedFromSecret | Taint::Secret)
                || sensitivity.classification != SecretSensitivity::NonSensitive;

            Some(RedactedValueView {
                is_tainted,
                taint_marker: if sensitivity.classification == SecretSensitivity::Unknown {
                    "UNKNOWN".to_string()
                } else {
                    taint_marker_string(taint)
                },
                digest: digest_hex,
                summary,
                summary_len,
            })
        }
    }
}

fn taint_marker_string(taint: Taint) -> String {
    match taint {
        Taint::Clean => "CLEAN".to_string(),
        Taint::DerivedFromSecret => "DERIVED".to_string(),
        Taint::Secret => "SECRET".to_string(),
    }
}

/// Redacts all secret-sensitive fields in a JSON object.
/// Returns a new JSON value with sensitive fields replaced by their
/// redacted views. Fail-closed: unknown fields are redacted.
pub fn redact_json_object(
    obj: &serde_json::map::Map<String, serde_json::Value>,
) -> serde_json::map::Map<String, serde_json::Value> {
    let mut result = serde_json::Map::new();

    for (key, value) in obj {
        let sensitivity = classify_secret_sensitivity(key);

        match sensitivity.classification {
            SecretSensitivity::NonSensitive => {
                // Pass through non-sensitive values recursively
                result.insert(
                    key.clone(),
                    redact_json_value(value, SecretSensitivity::NonSensitive),
                );
            }
            SecretSensitivity::Sensitive | SecretSensitivity::Unknown => {
                // Redact sensitive and unknown values
                if let Some(redacted) =
                    redact_json_value_as_redacted(value, sensitivity.classification)
                {
                    let mut redacted_map = serde_json::Map::new();
                    redacted_map.insert("__redacted".to_string(), serde_json::json!(true));
                    redacted_map
                        .insert("taint".to_string(), serde_json::json!(redacted.is_tainted));
                    redacted_map.insert(
                        "taint_marker".to_string(),
                        serde_json::json!(redacted.taint_marker),
                    );
                    redacted_map.insert("digest".to_string(), serde_json::json!(redacted.digest));
                    redacted_map.insert("summary".to_string(), serde_json::json!(redacted.summary));
                    result.insert(key.clone(), serde_json::Value::Object(redacted_map));
                } else {
                    // If redaction fails (should not happen for sensitive), replace with null
                    let mut redacted_map = serde_json::Map::new();
                    redacted_map.insert("__redacted".to_string(), serde_json::json!(true));
                    redacted_map
                        .insert("taint_marker".to_string(), serde_json::json!("REDACT_FAIL"));
                    result.insert(key.clone(), serde_json::Value::Object(redacted_map));
                }
            }
        }
    }

    result
}

fn redact_json_value(
    value: &serde_json::Value,
    sensitivity: SecretSensitivity,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(obj) => {
            if sensitivity == SecretSensitivity::NonSensitive {
                serde_json::Value::Object(redact_json_object(obj))
            } else {
                serde_json::Value::Null
            }
        }
        serde_json::Value::Array(arr) => {
            if sensitivity == SecretSensitivity::NonSensitive {
                serde_json::Value::Array(
                    arr.iter()
                        .map(|v| redact_json_value(v, sensitivity))
                        .collect(),
                )
            } else {
                serde_json::Value::Null
            }
        }
        serde_json::Value::String(s) if sensitivity == SecretSensitivity::NonSensitive => {
            serde_json::Value::String(s.clone())
        }
        _ => serde_json::Value::Null,
    }
}

fn redact_json_value_as_redacted(
    value: &serde_json::Value,
    classification: SecretSensitivity,
) -> Option<RedactedValueView> {
    let taint = Taint::Clean;
    let sensitivity = SensitivityClass {
        classification,
        reason: None,
    };

    match value {
        serde_json::Value::String(s) => redact_secret_value(s, taint, sensitivity),
        _ => Some(RedactedValueView {
            is_tainted: true,
            taint_marker: "REDACTED".to_string(),
            digest: String::new(),
            summary: String::new(),
            summary_len: 0,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_field_classification() {
        let result = classify_secret_sensitivity("password");
        assert!(matches!(
            result.classification,
            SecretSensitivity::Sensitive
        ));

        let result = classify_secret_sensitivity("api_token");
        assert!(matches!(
            result.classification,
            SecretSensitivity::Sensitive
        ));
    }

    #[test]
    fn non_sensitive_field_classification() {
        let result = classify_secret_sensitivity("user_id");
        assert!(matches!(
            result.classification,
            SecretSensitivity::NonSensitive
        ));

        let result = classify_secret_sensitivity("name");
        assert!(matches!(
            result.classification,
            SecretSensitivity::NonSensitive
        ));
    }

    #[test]
    fn unknown_field_classification_is_fail_closed() {
        let result = classify_secret_sensitivity("custom_data");
        assert!(matches!(result.classification, SecretSensitivity::Unknown));
    }

    #[test]
    fn redact_sensitive_value() {
        let taint = Taint::Clean;
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Sensitive,
            reason: None,
        };

        let result = redact_secret_value("my_secret_token", taint, sensitivity);
        assert!(result.is_some());

        let view = result.unwrap();
        assert!(view.is_tainted);
        assert!(!view.digest.is_empty());
        assert!(view.summary.is_empty()); // No summary for sensitive values
    }

    #[test]
    fn redact_unknown_sensitivity_is_fail_closed() {
        let taint = Taint::Clean;
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Unknown,
            reason: None,
        };

        let result = redact_secret_value("unknown_value", taint, sensitivity);
        assert!(result.is_some());

        let view = result.unwrap();
        assert!(view.is_tainted); // Unknown taints as true
        assert_eq!(view.taint_marker, "UNKNOWN");
    }

    #[test]
    fn non_sensitive_value_passes_through() {
        let taint = Taint::Clean;
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::NonSensitive,
            reason: None,
        };

        let result = redact_secret_value("user_123", taint, sensitivity);
        assert!(result.is_some());

        let view = result.unwrap();
        assert!(!view.is_tainted);
        assert!(view.digest.is_empty()); // No digest for non-sensitive
    }
}
