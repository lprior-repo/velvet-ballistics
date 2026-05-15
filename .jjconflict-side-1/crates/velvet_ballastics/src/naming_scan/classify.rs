use super::allowlist::exact_exception;
use super::legacy::{class_for_text, contains_legacy};
use super::types::*;

pub fn classify_occurrence(
    _path: RepoPath,
    _line: LineNumber,
    _column: ColumnNumber,
    text: &str,
    config: &ScanConfig,
) -> Result<OccurrenceClass, NamingScanError> {
    if text.is_empty() {
        return Ok(OccurrenceClass::NoOccurrence);
    }
    if let Some(canonical) = canonical_occurrence(text, &config.canonical_table) {
        return Ok(canonical);
    }
    if let Some(exception) = exact_exception(text, config) {
        return Ok(OccurrenceClass::AllowedLegacy { exception });
    }
    if contains_legacy(text) {
        return Ok(class_for_text(text));
    }
    Ok(OccurrenceClass::NoOccurrence)
}

fn canonical_occurrence(text: &str, table: &CanonicalSpellingTable) -> Option<OccurrenceClass> {
    if text == table.language_version {
        return Some(canonical_language_version(text));
    }
    if text == table.crate_module || text == table.bead_database {
        return Some(canonical_crate_module(text));
    }
    if text == table.product
        || text == table.binary
        || text == table.package
        || text == table.bead_rig
    {
        return Some(canonical_product(text));
    }
    None
}

fn canonical_language_version(text: &str) -> OccurrenceClass {
    OccurrenceClass::CanonicalLanguageVersion {
        canonical: text.to_owned(),
        kind: CanonicalNameKind::LanguageVersion,
    }
}

fn canonical_crate_module(text: &str) -> OccurrenceClass {
    OccurrenceClass::CanonicalCrateModule {
        canonical: text.to_owned(),
        kind: CanonicalNameKind::CrateModule,
    }
}

fn canonical_product(text: &str) -> OccurrenceClass {
    OccurrenceClass::CanonicalProduct {
        canonical: text.to_owned(),
        kind: CanonicalNameKind::Product,
    }
}
