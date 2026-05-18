//! Helper function tests for expression evaluation.
//!
//! This module re-exports edge case tests for the helper functions defined in
//! `vb_core::engine::expr_eval::ops_text_list` and `vb_core::engine::expr_eval::ops`.
//!
//! The 12 helpers covered are:
//! - `eval_contains` — text substring check
//! - `eval_starts_with` — text prefix check
//! - `eval_ends_with` — text suffix check
//! - `eval_has` — list membership check
//! - `eval_length` — length of text/list/object
//! - `eval_empty` — emptiness check for text/list/object/null
//! - `eval_sum` — sum of i64 list
//! - `eval_count` — count of list elements
//! - `eval_append` — append item to list (immutable)
//! - `eval_append_if` — conditional append
//! - `eval_unique` — deduplicate list preserving order
//! - `eval_merge` — merge two objects (right wins on conflict)

pub mod tests;
