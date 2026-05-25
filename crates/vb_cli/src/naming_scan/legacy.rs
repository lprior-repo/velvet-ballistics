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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_legacy_returns_true_for_legacy_project() {
        assert!(contains_legacy("velvet-ballastics is bad"));
    }

    #[test]
    fn contains_legacy_returns_true_for_legacy_crate() {
        assert!(contains_legacy("velvet_ballastics crate"));
    }

    #[test]
    fn contains_legacy_returns_true_for_legacy_language_version() {
        assert!(contains_legacy("velvet-ballastics/v1"));
    }

    #[test]
    fn contains_legacy_returns_true_for_pascal_case_legacy() {
        assert!(contains_legacy("Velvet-Ballastics rule"));
    }

    #[test]
    fn contains_legacy_returns_true_for_uppercase_legacy() {
        assert!(contains_legacy("VELVET-BALLASTICS"));
    }

    #[test]
    fn contains_legacy_returns_false_for_clean_text() {
        assert!(!contains_legacy("this is clean"));
    }

    #[test]
    fn contains_legacy_returns_false_for_canonical_hyphen() {
        assert!(!contains_legacy("velvet-ballistics"));
    }

    #[test]
    fn class_for_text_returns_legacy_crate_module_spelling() {
        let result = class_for_text("velvet_ballastics is wrong");
        match result {
            OccurrenceClass::InvalidLegacy {
                spelling_class,
                remediation,
            } => {
                assert_eq!(spelling_class, SpellingClass::LegacyCrateModuleSpelling);
                assert_eq!(remediation, CANONICAL_UNDERSCORE);
            }
            _ => panic!("wrong occurrence class"),
        }
    }

    #[test]
    fn class_for_text_returns_legacy_language_version_spelling() {
        let result = class_for_text("use velvet-ballastics/v1 here");
        match result {
            OccurrenceClass::InvalidLegacy {
                spelling_class,
                remediation,
            } => {
                assert_eq!(spelling_class, SpellingClass::LegacyLanguageVersionSpelling);
                assert_eq!(remediation, CANONICAL_LANGUAGE_VERSION);
            }
            _ => panic!("wrong occurrence class"),
        }
    }

    #[test]
    fn class_for_text_returns_legacy_project_spelling() {
        let result = class_for_text("velvet-ballistics is the old name");
        match result {
            OccurrenceClass::InvalidLegacy {
                spelling_class,
                remediation,
            } => {
                assert_eq!(spelling_class, SpellingClass::LegacyProjectSpelling);
                assert_eq!(remediation, CANONICAL_HYPHEN);
            }
            _ => panic!("wrong occurrence class"),
        }
    }
}
