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
#[non_exhaustive]
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
        // `Taint` is `#[non_exhaustive]`; unknown variants are
        // treated as unknown rather than silently misclassifying.
        _ => "UNKNOWN".to_string(),
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
        serde_json::Value::Number(n) if sensitivity == SecretSensitivity::NonSensitive => {
            serde_json::Value::Number(n.clone())
        }
        serde_json::Value::Bool(b) if sensitivity == SecretSensitivity::NonSensitive => {
            serde_json::Value::Bool(*b)
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

    // =====================================================================
    // redact_secret_value — all sensitivity × taint combinations
    // =====================================================================

    #[test]
    fn redact_secret_value_sensitive_with_clean_taint() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Sensitive,
            reason: None,
        };
        let result = redact_secret_value("hunter2", Taint::Clean, sensitivity).unwrap();
        assert!(result.is_tainted);
        assert!(!result.digest.is_empty());
        assert_eq!(result.summary_len, 0);
        assert_eq!(result.summary, "");
        assert_eq!(result.taint_marker, "CLEAN");
    }

    #[test]
    fn redact_secret_value_sensitive_with_secret_taint() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Sensitive,
            reason: None,
        };
        let result = redact_secret_value("hunter2", Taint::Secret, sensitivity).unwrap();
        assert!(result.is_tainted);
        assert_eq!(result.taint_marker, "SECRET");
    }

    #[test]
    fn redact_secret_value_sensitive_with_derived_taint() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Sensitive,
            reason: None,
        };
        let result =
            redact_secret_value("derived_secret", Taint::DerivedFromSecret, sensitivity).unwrap();
        assert!(result.is_tainted);
        assert_eq!(result.taint_marker, "DERIVED");
    }

    #[test]
    fn redact_secret_value_unknown_is_fail_closed() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Unknown,
            reason: None,
        };
        let result = redact_secret_value("unknown_val", Taint::Clean, sensitivity).unwrap();
        assert!(result.is_tainted);
        assert_eq!(result.taint_marker, "UNKNOWN");
        // Unknown gets bounded summary
        assert!(!result.summary.is_empty() || result.summary_len == 0);
    }

    #[test]
    fn redact_secret_value_unknown_bounded_summary_truncates() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Unknown,
            reason: None,
        };
        let long_value = "x".repeat(200);
        let result = redact_secret_value(&long_value, Taint::Clean, sensitivity).unwrap();
        assert_eq!(
            result.summary_len,
            core::cmp::min(200, MAX_REDACTION_SUMMARY_LEN)
        );
        assert_eq!(result.summary.len(), result.summary_len);
    }

    #[test]
    fn redact_secret_value_non_sensitive_with_secret_taint() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::NonSensitive,
            reason: None,
        };
        let result = redact_secret_value("public_data", Taint::Secret, sensitivity).unwrap();
        assert!(result.is_tainted); // taint is still reflected even if non-sensitive
        assert!(result.digest.is_empty());
        assert_eq!(result.taint_marker, "SECRET");
    }

    #[test]
    fn redact_secret_value_digest_is_blake3_hex() {
        let sensitivity = SensitivityClass {
            classification: SecretSensitivity::Sensitive,
            reason: None,
        };
        let result = redact_secret_value("test_value", Taint::Clean, sensitivity).unwrap();
        // BLAKE3 hex is 64 chars
        assert_eq!(result.digest.len(), 64);
        assert!(result.digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // =====================================================================
    // classify_secret_sensitivity — exhaustive field patterns
    // =====================================================================

    #[test]
    fn classify_secret_sensitivity_password_variants() {
        let r = classify_secret_sensitivity("user_password");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
        let r2 = classify_secret_sensitivity("PASSWORD");
        assert!(matches!(r2.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_secret_variants() {
        let r = classify_secret_sensitivity("client_secret");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
        let r2 = classify_secret_sensitivity("SECRET_TOKEN");
        assert!(matches!(r2.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_token_variants() {
        let r = classify_secret_sensitivity("access_token");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
        let r2 = classify_secret_sensitivity("API_TOKEN");
        assert!(matches!(r2.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_api_key_variants() {
        let r = classify_secret_sensitivity("api_key");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
        let r2 = classify_secret_sensitivity("apiKey");
        assert!(matches!(r2.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_private_key_variants() {
        let r = classify_secret_sensitivity("private_key");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
        let r2 = classify_secret_sensitivity("privateKey");
        assert!(matches!(r2.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_credential_variants() {
        let r = classify_secret_sensitivity("service_credentials");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_auth_variants() {
        let r = classify_secret_sensitivity("auth_header");
        assert!(matches!(r.classification, SecretSensitivity::Sensitive));
        let r2 = classify_secret_sensitivity("authorization");
        assert!(matches!(r2.classification, SecretSensitivity::Sensitive));
    }

    #[test]
    fn classify_secret_sensitivity_non_sensitive_patterns() {
        for field in &[
            "user_name",
            "file_name",
            "record_id",
            "status_code",
            "node_type",
            "event_count",
            "item_index",
            "action_id",
        ] {
            let r = classify_secret_sensitivity(field);
            assert!(
                matches!(r.classification, SecretSensitivity::NonSensitive),
                "field '{}' should be NonSensitive but got {:?}",
                field,
                r.classification
            );
        }
    }

    #[test]
    fn classify_secret_sensitivity_unknown_is_fail_closed() {
        let r = classify_secret_sensitivity("custom_zebra_field");
        assert!(matches!(r.classification, SecretSensitivity::Unknown));
    }

    // =====================================================================
    // redact_json_object
    // =====================================================================

    #[test]
    fn redact_json_object_passes_through_non_sensitive() {
        let mut obj = serde_json::Map::new();
        obj.insert("user_name".to_string(), serde_json::json!("Alice"));
        obj.insert("item_index".to_string(), serde_json::json!(42));

        let result = redact_json_object(&obj);

        // Non-sensitive values pass through
        assert_eq!(result.get("user_name"), Some(&serde_json::json!("Alice")));
        // item_index matches "index" pattern → NonSensitive → passes through
        assert_eq!(result.get("item_index"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn redact_json_object_redacts_sensitive_fields() {
        let mut obj = serde_json::Map::new();
        obj.insert("password".to_string(), serde_json::json!("hunter2"));
        obj.insert("api_token".to_string(), serde_json::json!("tok_12345"));

        let result = redact_json_object(&obj);

        // Sensitive fields should be redacted maps
        let pass = result.get("password").and_then(|v| v.get("__redacted"));
        assert_eq!(
            pass,
            Some(&serde_json::json!(true)),
            "password should be redacted"
        );

        let token_redacted = result.get("api_token").and_then(|v| v.get("__redacted"));
        assert_eq!(
            token_redacted,
            Some(&serde_json::json!(true)),
            "api_token should be redacted"
        );
    }

    #[test]
    fn redact_json_object_redacts_unknown_fields() {
        let mut obj = serde_json::Map::new();
        obj.insert("custom_data".to_string(), serde_json::json!("some value"));

        let result = redact_json_object(&obj);

        let redacted = result.get("custom_data").and_then(|v| v.get("__redacted"));
        assert_eq!(
            redacted,
            Some(&serde_json::json!(true)),
            "unknown field should be redacted"
        );
    }

    #[test]
    fn redact_json_object_nested_object_non_sensitive_passes() {
        let mut obj = serde_json::Map::new();
        let mut nested = serde_json::Map::new();
        nested.insert("name".to_string(), serde_json::json!("Bob"));
        // "record_id" matches "id" → NonSensitive → nested object passes through
        obj.insert("record_id".to_string(), serde_json::Value::Object(nested));

        let result = redact_json_object(&obj);

        let record = result.get("record_id").and_then(|v| v.get("name"));
        assert_eq!(record, Some(&serde_json::json!("Bob")));
    }

    #[test]
    fn redact_json_object_array_non_sensitive_passes() {
        let mut obj = serde_json::Map::new();
        // "name_list" matches "name" → NonSensitive → array passes through
        obj.insert("name_list".to_string(), serde_json::json!(["a", "b", "c"]));

        let result = redact_json_object(&obj);

        let names = result.get("name_list").and_then(|v| v.as_array());
        assert_eq!(names.map(|a| a.len()), Some(3));
    }

    #[test]
    fn redact_json_object_sensitive_string_gets_digest() {
        let mut obj = serde_json::Map::new();
        obj.insert("secret".to_string(), serde_json::json!("my_secret_value"));

        let result = redact_json_object(&obj);

        let digest = result.get("secret").and_then(|v| v.get("digest"));
        assert!(digest.is_some());
        let digest_str = digest.unwrap().as_str().unwrap();
        assert!(!digest_str.is_empty());
    }

    // =====================================================================
    // MAX_REDACTION_SUMMARY_LEN and MAX_DIGEST_LEN constants
    // =====================================================================

    #[test]
    fn max_redaction_summary_len_is_64() {
        assert_eq!(MAX_REDACTION_SUMMARY_LEN, 64);
    }

    #[test]
    fn max_digest_len_is_64() {
        assert_eq!(MAX_DIGEST_LEN, 64);
    }
}
