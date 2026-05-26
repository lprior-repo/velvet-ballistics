use std::cmp::Ordering;

use super::types::*;

pub(crate) fn compare_finding(left: &NamingFinding, right: &NamingFinding) -> Ordering {
    (&left.path, left.line, left.column, &left.spelling_class).cmp(&(
        &right.path,
        right.line,
        right.column,
        &right.spelling_class,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(path: &str, line: u64, col: u64, sc: SpellingClass) -> NamingFinding {
        NamingFinding {
            path: RepoPath::new(path),
            line: LineNumber::new(line),
            column: ColumnNumber::new(col),
            spelling_class: sc,
            remediation: "fix".to_owned(),
        }
    }

    #[test]
    fn compare_finding_returns_equal_for_identical_findings() {
        let a = finding("a.rs", 1, 1, SpellingClass::LegacyProjectSpelling);
        let b = finding("a.rs", 1, 1, SpellingClass::LegacyProjectSpelling);
        assert_eq!(compare_finding(&a, &b), Ordering::Equal);
    }

    #[test]
    fn compare_finding_orders_by_path_first() {
        let a = finding("a.rs", 5, 1, SpellingClass::LegacyProjectSpelling);
        let b = finding("b.rs", 1, 5, SpellingClass::LegacyProjectSpelling);
        assert_eq!(compare_finding(&a, &b), Ordering::Less);
    }

    #[test]
    fn compare_finding_orders_by_line_second() {
        let a = finding("a.rs", 1, 10, SpellingClass::LegacyProjectSpelling);
        let b = finding("a.rs", 2, 1, SpellingClass::LegacyProjectSpelling);
        assert_eq!(compare_finding(&a, &b), Ordering::Less);
    }

    #[test]
    fn compare_finding_orders_by_column_third() {
        let a = finding("a.rs", 1, 1, SpellingClass::LegacyProjectSpelling);
        let b = finding("a.rs", 1, 2, SpellingClass::LegacyProjectSpelling);
        assert_eq!(compare_finding(&a, &b), Ordering::Less);
    }

    #[test]
    fn compare_finding_orders_by_spelling_class_fourth() {
        let a = finding("a.rs", 1, 1, SpellingClass::LegacyCrateModuleSpelling);
        let b = finding("a.rs", 1, 1, SpellingClass::LegacyProjectSpelling);
        assert!(compare_finding(&a, &b) != Ordering::Equal);
    }
}
