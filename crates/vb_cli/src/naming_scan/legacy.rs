use super::types::*;

pub(crate) fn contains_legacy(text: &str) -> bool {
    text.contains(LEGACY_PROJECT)
        || text.contains(LEGACY_CRATE)
        || text.contains(LEGACY_LANGUAGE_VERSION)
        || text.contains("Velvet-Ballastics")
        || text.contains("VELVET-BALLASTICS")
        || text.contains("velvet‐ballistics")
}

pub(crate) fn class_for_text(text: &str) -> OccurrenceClass {
    if text.contains(LEGACY_CRATE) {
        OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyCrateModuleSpelling,
            remediation: CANONICAL_UNDERSCORE.to_owned(),
        }
    } else if text.contains(LEGACY_LANGUAGE_VERSION) {
        OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyLanguageVersionSpelling,
            remediation: CANONICAL_LANGUAGE_VERSION.to_owned(),
        }
    } else {
        OccurrenceClass::InvalidLegacy {
            spelling_class: SpellingClass::LegacyProjectSpelling,
            remediation: CANONICAL_HYPHEN.to_owned(),
        }
    }
}
