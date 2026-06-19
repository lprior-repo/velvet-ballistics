/// Construction logic for the canonical spelling table.
///
/// Transforms a validated slice of [`CanonicalEntry`] into a
/// [`CanonicalSpellingTable`], enforcing uniqueness and token correctness.
use super::config_types::*;
use super::types::*;

/// Build a [`CanonicalSpellingTable`] from entries.
///
/// Returns an error if the entries contain duplicates, contradictions, or
/// are missing required kinds.
pub(crate) fn table_from_entries(
    entries: &[CanonicalEntry],
) -> Result<CanonicalSpellingTable, NamingScanError> {
    let mut seen = Vec::new();
    for entry in entries {
        if seen.contains(&entry.kind) {
            return duplicate_error(entry.kind);
        }
        seen.push(entry.kind);
        validate_entry(entry)?;
    }
    for kind in required_kinds() {
        if !seen.contains(&kind) {
            return missing_error(kind);
        }
    }
    Ok(canonical_spelling_table())
}

/// Validate a single [`CanonicalEntry`] against its expected token.
fn validate_entry(entry: &CanonicalEntry) -> Result<(), NamingScanError> {
    let expected = expected_token(entry.kind);
    if entry.token == expected {
        Ok(())
    } else {
        invalid_config(&format!(
            "contradictory token for {}: {}",
            kind_name(entry.kind),
            entry.token
        ))
    }
}

/// Construct the default [`CanonicalSpellingTable`].
#[must_use]
pub fn canonical_spelling_table() -> CanonicalSpellingTable {
    CanonicalSpellingTable {
        product: CANONICAL_HYPHEN.to_owned(),
        binary: CANONICAL_HYPHEN.to_owned(),
        package: CANONICAL_HYPHEN.to_owned(),
        bead_rig: CANONICAL_HYPHEN.to_owned(),
        crate_module: CANONICAL_UNDERSCORE.to_owned(),
        bead_database: CANONICAL_UNDERSCORE.to_owned(),
        language_version: CANONICAL_LANGUAGE_VERSION.to_owned(),
    }
}

fn duplicate_error(kind: CanonicalNameKind) -> Result<CanonicalSpellingTable, NamingScanError> {
    if kind == CanonicalNameKind::LanguageVersion {
        invalid_config("canonical kind count one above required: duplicate language_version")
    } else {
        invalid_config(&format!("duplicate canonical kind: {}", kind_name(kind)))
    }
}

fn missing_error(kind: CanonicalNameKind) -> Result<CanonicalSpellingTable, NamingScanError> {
    if kind == CanonicalNameKind::BeadDatabase {
        invalid_config("canonical kind count one below required: bead_database missing")
    } else {
        invalid_config(&format!("missing canonical kind: {}", kind_name(kind)))
    }
}

fn required_kinds() -> [CanonicalNameKind; 7] {
    [
        CanonicalNameKind::Product,
        CanonicalNameKind::Binary,
        CanonicalNameKind::Package,
        CanonicalNameKind::BeadRig,
        CanonicalNameKind::CrateModule,
        CanonicalNameKind::BeadDatabase,
        CanonicalNameKind::LanguageVersion,
    ]
}

fn kind_name(kind: CanonicalNameKind) -> &'static str {
    match kind {
        CanonicalNameKind::Product => "product",
        CanonicalNameKind::Binary => "binary",
        CanonicalNameKind::Package => "package",
        CanonicalNameKind::BeadRig => "bead_rig",
        CanonicalNameKind::CrateModule => "crate_module",
        CanonicalNameKind::BeadDatabase => "bead_database",
        CanonicalNameKind::LanguageVersion => "language_version",
    }
}

fn expected_token(kind: CanonicalNameKind) -> &'static str {
    match kind {
        CanonicalNameKind::CrateModule | CanonicalNameKind::BeadDatabase => CANONICAL_UNDERSCORE,
        CanonicalNameKind::LanguageVersion => CANONICAL_LANGUAGE_VERSION,
        CanonicalNameKind::Product
        | CanonicalNameKind::Binary
        | CanonicalNameKind::Package
        | CanonicalNameKind::BeadRig => CANONICAL_HYPHEN,
    }
}
