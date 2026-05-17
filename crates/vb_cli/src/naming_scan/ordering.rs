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
