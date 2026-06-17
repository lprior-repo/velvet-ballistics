#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

//! JSONL structured logging per crate/lane.
//!
//! Writes per-run logs to target/xtask-proof/&lt;run-id&gt;/&lt;crate&gt;/&lt;lane&gt;.jsonl

use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct LaneLogEntry {
    pub crate_name: String,
    pub lane: String,
    pub command: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub status: String,
    pub timestamp: String,
}

pub struct RunLogger {
    pub run_id: String,
    pub base_dir: PathBuf,
}

impl RunLogger {
    pub fn new(run_id: &str) -> Self {
        let base_dir = PathBuf::from("target").join("xtask-proof").join(run_id);
        RunLogger {
            run_id: run_id.to_string(),
            base_dir,
        }
    }

    pub fn log_entry(
        &self,
        crate_name: &str,
        lane: &str,
        command: &str,
        exit_code: Option<i32>,
        duration_ms: u64,
        status: &str,
    ) -> anyhow::Result<()> {
        let entry = LaneLogEntry {
            crate_name: crate_name.to_string(),
            lane: lane.to_string(),
            command: command.to_string(),
            exit_code,
            duration_ms,
            status: status.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };

        let dir = self.base_dir.join(crate_name);
        fs::create_dir_all(&dir)?;

        let file_path = dir.join(format!("{lane}.jsonl"));
        let json = serde_json::to_string(&entry)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;
        writeln!(file, "{json}")?;

        Ok(())
    }
}

pub fn generate_run_id() -> String {
    let now = Utc::now();
    format!("{}", now.format("%Y%m%d-%H%M%S"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_entry_serialization() {
        let entry = LaneLogEntry {
            crate_name: "vb_core".to_string(),
            lane: "test".to_string(),
            command: "cargo test -p vb_core".to_string(),
            exit_code: Some(0),
            duration_ms: 1234,
            status: "pass".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("vb_core"));
        assert!(json.contains("test"));
        assert!(json.contains("pass"));
    }

    #[test]
    fn test_generate_run_id() {
        let id = generate_run_id();
        assert!(!id.is_empty());
        assert!(id.len() > 10);
    }
}
