// Public API functions for evidence bundles.
//
// Types are in scope from include! directives in evidence.rs:
// Error, GateEvidence, GateStatus, GateStatusKind, Path, PathBuf, Serialize, Deserialize

// ── Schema Version Parser ─────────────────────────────────────────────────────

/// Parse a bundle schema version in major.minor form.
///
/// Format: "major.minor" where both parts are non-negative integers
/// without leading zeros (except "0" itself).
///
/// Returns the original string on success.
pub fn parse_bundle_schema_version(s: &str) -> std::result::Result<String, Error> {
    let parts: Vec<&str> = s.splitn(2, '.').collect();
    let [major_s, minor_s] = parts.as_slice() else {
        return Err(Error::SchemaVersionParseFailed { version: s.to_string() });
    };
    if major_s.is_empty()
        || minor_s.is_empty()
        || (major_s.len() > 1 && major_s.starts_with('0'))
        || (minor_s.len() > 1 && minor_s.starts_with('0'))
    {
        return Err(Error::SchemaVersionParseFailed { version: s.to_string() });
    }
    let major = major_s
        .parse::<u64>()
        .map_err(|_| Error::SchemaVersionParseFailed { version: s.to_string() })?;
    if major > 1 {
        return Err(Error::SchemaVersionParseFailed { version: s.to_string() });
    }
    let _minor = minor_s
        .parse::<u64>()
        .map_err(|_| Error::SchemaVersionParseFailed { version: s.to_string() })?;
    Ok(s.to_string())
}

// ── Bundle Validator ─────────────────────────────────────────────────────────

/// Validate a deserialised bundle's required fields.
///
/// Returns an empty vec if the bundle is valid.
/// Returns one `Error::MissingRequiredField` per absent required field.
pub fn validate_bundle(bundle: &EvidenceBundle) -> Vec<Error> {
    let mut errors = Vec::new();
    check_schema_version(bundle, &mut errors);
    check_linked_bead_id(bundle, &mut errors);
    check_executor_context(bundle, &mut errors);
    errors
}

fn check_schema_version(bundle: &EvidenceBundle, errors: &mut Vec<Error>) {
    if bundle.schema_version.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "schema_version".to_string(),
        });
    } else if parse_bundle_schema_version(&bundle.schema_version).is_err() {
        errors.push(Error::SchemaVersionParseFailed {
            version: bundle.schema_version.clone(),
        });
    }
}

fn check_linked_bead_id(bundle: &EvidenceBundle, errors: &mut Vec<Error>) {
    if bundle.linked_bead_id.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "linked_bead_id".to_string(),
        });
    }
}

fn check_executor_context(bundle: &EvidenceBundle, errors: &mut Vec<Error>) {
    let ctx = &bundle.executor_context;
    if ctx.agent.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.agent".to_string(),
        });
    }
    if ctx.timestamp.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.timestamp".to_string(),
        });
    }
    if ctx.machine.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.machine".to_string(),
        });
    }
}

// ── YAML Serialization Helpers ─────────────────────────────────────────────────

fn serialize_yaml_bundle(
    bundle: &EvidenceBundle,
    formatted_format: &str,
) -> std::result::Result<Vec<u8>, Error> {
    let mut yaml =
        serde_json::to_string_pretty(bundle).map_err(|e| Error::BundleSerializationFailed {
            format: formatted_format.to_string(),
            cause: e.to_string(),
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

// ── Write Bundle ──────────────────────────────────────────────────────────────

/// Serialise and write an `EvidenceBundle` to disk.
///
/// Creates parent directories if they do not exist.
pub fn write_bundle(
    bundle: &EvidenceBundle,
    path: &Path,
    format: EvidenceBundleFormat,
) -> std::result::Result<(), Error> {
    create_parent_dirs(path)?;
    let bytes = serialize_bundle(bundle, format)?;
    std::fs::write(path, &bytes).map_err(|e| Error::EvidenceWriteFailed {
        gate: "bundle".to_string(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })
}

fn create_parent_dirs(path: &Path) -> std::result::Result<(), Error> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(parent).map_err(|e| {
        let bead = parent.to_string_lossy().to_string();
        Error::BeadDirectoryCreationFailed {
            bead,
            cause: e.to_string(),
        }
    })?;
    Ok(())
}

fn serialize_bundle(
    bundle: &EvidenceBundle,
    format: EvidenceBundleFormat,
) -> std::result::Result<Vec<u8>, Error> {
    let formatted_format = format_to_string(format);
    match format {
        EvidenceBundleFormat::Yaml => serialize_yaml_bundle(bundle, &formatted_format),
        EvidenceBundleFormat::Json => serde_json::to_string(bundle)
            .map(|s| s.into_bytes())
            .map_err(|e| Error::BundleSerializationFailed {
                format: formatted_format,
                cause: e.to_string(),
            }),
        EvidenceBundleFormat::Postcard => {
            let wire = EvidenceBundlePostcard::from_bundle(bundle);
            postcard::to_allocvec(&wire).map_err(|e| Error::BundleSerializationFailed {
                format: formatted_format,
                cause: e.to_string(),
            })
        }
    }
}

// ── Read Bundle ──────────────────────────────────────────────────────────────

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
        EvidenceBundleFormat::Json => serde_json::from_slice(&contents)
            .map_err(|e| Error::BundleSerializationFailed { format: "json".to_string(), cause: e.to_string() }),
        EvidenceBundleFormat::Postcard => {
            let wire: EvidenceBundlePostcard = postcard::from_bytes(&contents)
                .map_err(|e| Error::BundleSerializationFailed { format: "postcard".to_string(), cause: e.to_string() })?;
            Ok(wire.into_bundle())
        }
    }
}
