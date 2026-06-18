#![forbid(unsafe_code)]
//! Shared helpers used by the lexer and parser.

use crate::expression::domain::ExpressionHelper;

/// Resolves a helper identifier to its domain variant.
pub(crate) fn parse_helper(name: &str) -> Option<ExpressionHelper> {
    match name {
        "contains" => Some(ExpressionHelper::Contains),
        "starts_with" => Some(ExpressionHelper::StartsWith),
        "ends_with" => Some(ExpressionHelper::EndsWith),
        "has" => Some(ExpressionHelper::Has),
        "exists" => Some(ExpressionHelper::Exists),
        "length" => Some(ExpressionHelper::Length),
        "empty" => Some(ExpressionHelper::Empty),
        "append" => Some(ExpressionHelper::Append),
        "append_if" => Some(ExpressionHelper::AppendIf),
        "merge" => Some(ExpressionHelper::Merge),
        "sum" => Some(ExpressionHelper::Sum),
        "count" => Some(ExpressionHelper::Count),
        "unique" => Some(ExpressionHelper::Unique),
        "coalesce" => Some(ExpressionHelper::Coalesce),
        _ => None,
    }
}

// ── Character predicates ─────────────────────────────────────────────────────

pub(super) fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

pub(super) fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub(super) fn is_reference_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}
