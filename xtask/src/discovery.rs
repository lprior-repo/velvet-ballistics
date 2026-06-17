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

//! Workspace crate discovery via cargo metadata.
//!
//! Calls `cargo metadata --no-deps --format-version 1` exactly once
//! and parses the output into `CrateInfo` records.

use cargo_metadata::{Metadata, MetadataCommand, Package};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<String>,
}

pub fn discover_crates(workspace_root: &std::path::Path) -> anyhow::Result<Vec<CrateInfo>> {
    let metadata = run_cargo_metadata(workspace_root)?;
    Ok(parse_crates(&metadata))
}

fn run_cargo_metadata(workspace_root: &std::path::Path) -> anyhow::Result<Metadata> {
    MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .no_deps()
        .exec()
        .map_err(|e| anyhow::anyhow!("cargo metadata failed: {e}"))
}

fn parse_crates(metadata: &Metadata) -> Vec<CrateInfo> {
    let workspace_members: std::collections::HashSet<_> = metadata
        .workspace_members
        .iter()
        .map(|id| id.repr.as_str())
        .collect();

    metadata
        .packages
        .iter()
        .filter(|pkg| workspace_members.contains(pkg.id.repr.as_str()))
        .filter(|pkg| pkg.name != "xtask")
        .filter(|pkg| pkg.name != "workspace_tests")
        .filter(|pkg| pkg.name != "vb_benchmark")
        .map(pkg_to_crate_info)
        .collect()
}

fn pkg_to_crate_info(pkg: &Package) -> CrateInfo {
    let deps: Vec<String> = pkg
        .dependencies
        .iter()
        .filter(|d| d.path.is_some())
        .map(|d| d.name.clone())
        .collect();

    CrateInfo {
        name: pkg.name.clone(),
        manifest_path: pkg.manifest_path.clone().into(),
        dependencies: deps,
    }
}

pub fn filter_crates(
    crates: &[CrateInfo],
    include: Option<&[String]>,
    exclude: Option<&[String]>,
) -> Vec<CrateInfo> {
    crates
        .iter()
        .filter(|c| match include {
            Some(patterns) => matches_any(&c.name, patterns),
            None => true,
        })
        .filter(|c| match exclude {
            Some(patterns) => !matches_any(&c.name, patterns),
            None => true,
        })
        .cloned()
        .collect()
}

fn matches_any(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| name.contains(p.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_crates_include() {
        let crates = make_test_crates(&["vb_core", "vb_cli", "vb_storage"]);
        let filtered = filter_crates(&crates, Some(&["vb_core".to_string()]), None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "vb_core");
    }

    #[test]
    fn test_filter_crates_exclude() {
        let crates = make_test_crates(&["vb_core", "vb_cli", "vb_storage"]);
        let filtered = filter_crates(&crates, None, Some(&["vb_cli".to_string()]));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_crates_include_and_exclude() {
        let crates = make_test_crates(&["vb_core", "vb_cli", "vb_storage"]);
        let filtered = filter_crates(
            &crates,
            Some(&["vb_".to_string()]),
            Some(&["vb_cli".to_string()]),
        );
        assert_eq!(filtered.len(), 2);
    }

    fn make_test_crates(names: &[&str]) -> Vec<CrateInfo> {
        names
            .iter()
            .map(|n| CrateInfo {
                name: n.to_string(),
                manifest_path: PathBuf::from(format!("crates/{n}/Cargo.toml")),
                dependencies: vec![],
            })
            .collect()
    }
}
