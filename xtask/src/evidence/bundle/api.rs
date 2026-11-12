// ── Public API ────────────────────────────────────────────────────────────────

/// Construct the bundle file path for a given bead and format.
///
/// Path is `.evidence/<bead-id>/bundle.<ext>`.
pub fn bundle_path(bead_id: &str, format: EvidenceBundleFormat) -> PathBuf {
    PathBuf::from(".evidence")
        .join(bundle_path_component(bead_id))
        .join(format!("bundle.{}", format.extension()))
}

fn bundle_path_component(bead_id: &str) -> String {
    let component: String = bead_id
        .chars()
        .map(|ch| match ch {
            '/' | '\\' => '_',
            other => other,
        })
        .collect();

    if component.is_empty() {
        String::from("unknown")
    } else {
        component
    }
}

/// Parse a bundle schema version in major.minor form.
///
/// Format: "major.minor" where both parts are non-negative integers
/// without leading zeros (except "0" itself).
///
/// Returns the original string on success.
pub fn parse_bundle_schema_version(s: &str) -> std::result::Result<String, Error> {
    if s.is_empty() {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    let parts: Vec<&str> = s.splitn(2, '.').collect();
    // safety: splitn(2) returns at most 2 parts; length check below guards indexing
    #[allow(clippy::indexing_slicing)]
    if parts.len() != 2
        || parts[0].is_empty()
        || parts[1].is_empty()
    {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    #[allow(clippy::indexing_slicing)]
    let (major_s, minor_s) = (parts[0], parts[1]);

    // Reject leading zeros: "0" is OK, but "00", "01", etc. are not.
    let reject_leading_zero = |s: &str| -> bool {
        s.len() > 1 && s.starts_with('0')
    };

    if reject_leading_zero(major_s) || reject_leading_zero(minor_s) {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    let major: u64 = match major_s.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::SchemaVersionParseFailed {
                version: s.to_string(),
            });
        }
    };
    let _minor: u64 = match minor_s.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::SchemaVersionParseFailed {
                version: s.to_string(),
            });
        }
    };

    // Reject major > 1 (consumer supports only major version 0 and 1).
    if major > 1 {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    Ok(s.to_string())
}

/// Validate a deserialised bundle's required fields.
///
/// Returns an empty vec if the bundle is valid.
/// Returns one `Error::MissingRequiredField` per absent required field.
pub fn validate_bundle(bundle: &EvidenceBundle) -> Vec<Error> {
    let mut errors: Vec<Error> = Vec::new();

    // Schema version: non-empty and parseable.
    if bundle.schema_version.is_empty()
        || parse_bundle_schema_version(&bundle.schema_version).is_err()
    {
        if bundle.schema_version.is_empty() {
            errors.push(Error::MissingRequiredField {
                field: "schema_version".to_string(),
            });
        } else {
            errors.push(Error::SchemaVersionParseFailed {
                version: bundle.schema_version.clone(),
            });
        }
    }

    // Linked bead ID: non-empty.
    if bundle.linked_bead_id.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "linked_bead_id".to_string(),
        });
    }

    // Executor context: all three sub-fields non-empty.
    if bundle.executor_context.agent.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.agent".to_string(),
        });
    }
    if bundle.executor_context.timestamp.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.timestamp".to_string(),
        });
    }
    if bundle.executor_context.machine.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.machine".to_string(),
        });
    }

    errors
}

/// Serialise and write an `EvidenceBundle` to disk.
///
/// Creates parent directories if they do not exist.
pub fn write_bundle(
    bundle: &EvidenceBundle,
    path: &Path,
    format: EvidenceBundleFormat,
) -> std::result::Result<(), Error> {
    // Create parent directories.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::BeadDirectoryCreationFailed {
            bead: path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string(),
            cause: e.to_string(),
        })?;
    }

    let formatted_format = format_to_string(format);

    let serialised: std::result::Result<Vec<u8>, Error> = match format {
        EvidenceBundleFormat::Yaml => serialize_yaml_bundle(bundle, &formatted_format),
        EvidenceBundleFormat::Json => {
            let json = serde_json::to_string(bundle).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: formatted_format.clone(),
                    cause: e.to_string(),
                }
            })?;
            Ok(json.into_bytes())
        }
        EvidenceBundleFormat::Postcard => {
            let wire = EvidenceBundlePostcard::from_bundle(bundle);
            let bytes = postcard::to_allocvec(&wire).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: formatted_format.clone(),
                    cause: e.to_string(),
                }
            })?;
            Ok(bytes)
        }
    };

    let bytes = serialised?;
    std::fs::write(path, &bytes).map_err(|e| Error::EvidenceWriteFailed {
        gate: "bundle".to_string(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })
}

/// Deserialise an `EvidenceBundle` from a file.
pub fn read_bundle(
    path: &Path,
    format: EvidenceBundleFormat,
) -> std::result::Result<EvidenceBundle, Error> {
    let contents = std::fs::read(path).map_err(|e| Error::EvidenceWriteFailed {
        gate: "bundle".to_string(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })?;

    match format {
        EvidenceBundleFormat::Yaml => deserialize_yaml_bundle(&contents),
        EvidenceBundleFormat::Json => {
            let bundle: EvidenceBundle = serde_json::from_slice(&contents).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: "json".to_string(),
                    cause: e.to_string(),
                }
            })?;
            Ok(bundle)
        }
        EvidenceBundleFormat::Postcard => {
            let wire: EvidenceBundlePostcard = postcard::from_bytes(&contents).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: "postcard".to_string(),
                    cause: e.to_string(),
                }
            })?;
            Ok(wire.into_bundle())
        }
    }
}

fn serialize_yaml_bundle(
    bundle: &EvidenceBundle,
    formatted_format: &str,
) -> std::result::Result<Vec<u8>, Error> {
    // serde-saphyr 0.0.25 currently emits some scalars whose trailing spaces
    // are stripped by its own parser on round-trip. JSON is valid YAML 1.2, so
    // emit a JSON-compatible YAML document to preserve arbitrary evidence text.
    let mut yaml = serde_json::to_string_pretty(bundle).map_err(|e| {
        Error::BundleSerializationFailed {
            format: formatted_format.to_string(),
            cause: e.to_string(),
        }
    })?;
    yaml.push('\n');
    Ok(yaml.into_bytes())
}

fn deserialize_yaml_bundle(contents: &[u8]) -> std::result::Result<EvidenceBundle, Error> {
    match serde_saphyr::from_slice(contents) {
        Ok(bundle) => Ok(bundle),
        Err(yaml_error) => serde_json::from_slice(contents).map_err(|json_error| {
            Error::BundleSerializationFailed {
                format: "yaml".to_string(),
                cause: format!(
                    "serde-saphyr parse failed: {}; json-compatible yaml parse failed: {}",
                    yaml_error, json_error
                ),
            }
        }),
    }
}

