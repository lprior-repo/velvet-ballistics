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

//! Proof/test lane definitions and command generation.
//!
//! Each lane maps to a shell command with crate-specific arguments.

use std::path::Path;

#[derive(Debug, Clone)]
pub struct Lane {
    pub name: String,
    pub required: bool,
}

pub fn lane_command(lane: &Lane, crate_name: &str, workspace_root: &Path) -> Vec<String> {
    match lane.name.as_str() {
        "test" => vec![
            "cargo".into(),
            "test".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "clippy" => vec![
            "cargo".into(),
            "clippy".into(),
            "-p".into(),
            crate_name.into(),
            "--".into(),
            "-D".into(),
            "warnings".into(),
        ],
        "nextest" => vec![
            "cargo".into(),
            "nextest".into(),
            "run".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "kani" => vec![
            "cargo".into(),
            "kani".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "miri" => vec![
            "cargo".into(),
            "+nightly".into(),
            "miri".into(),
            "test".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "loom" => vec![
            "cargo".into(),
            "test".into(),
            "-p".into(),
            crate_name.into(),
            "--features".into(),
            "loom".into(),
        ],
        "fuzz" => vec![
            "cargo".into(),
            "fuzz".into(),
            "run".into(),
            format!("{crate_name}_fuzz"),
        ],
        "mutants" => vec![
            "cargo".into(),
            "mutants".into(),
            "-p".into(),
            crate_name.into(),
            "--no-times".into(),
        ],
        "coverage" => vec![
            "cargo".into(),
            "llvm-cov".into(),
            "--no-report".into(),
            "-p".into(),
            crate_name.into(),
        ],
        "verus" => verus_command(crate_name, workspace_root),
        "tla" => tla_command(crate_name, workspace_root),
        "flux" => vec![
            "cargo".into(),
            "flux".into(),
            "-p".into(),
            crate_name.into(),
        ],
        _ => vec!["echo".into(), format!("unknown lane: {}", lane.name)],
    }
}

fn verus_command(crate_name: &str, workspace_root: &Path) -> Vec<String> {
    let verus_dir = workspace_root.join("verification").join("verus");
    vec![
        "verus".into(),
        format!("{}/{crate_name}.rs", verus_dir.display()),
    ]
}

fn tla_command(crate_name: &str, workspace_root: &Path) -> Vec<String> {
    let tla_file = workspace_root
        .join("verification")
        .join("tla")
        .join(format!("{crate_name}.tla"));
    vec!["tla2tools".into(), format!("{}", tla_file.display())]
}

pub fn detect_available_lanes(workspace_root: &Path) -> Vec<Lane> {
    let all_lanes = [
        ("test", true),
        ("clippy", true),
        ("nextest", false),
        ("kani", false),
        ("miri", false),
        ("loom", false),
        ("fuzz", false),
        ("mutants", false),
        ("coverage", false),
        ("verus", false),
        ("tla", false),
        ("flux", false),
    ];

    all_lanes
        .iter()
        .filter(|(name, required)| {
            if *required {
                true
            } else {
                is_tool_available(name, workspace_root)
            }
        })
        .map(|(name, required)| Lane {
            name: name.to_string(),
            required: *required,
        })
        .collect()
}

fn is_tool_available(lane: &str, workspace_root: &Path) -> bool {
    match lane {
        "nextest" => tool_in_path("cargo-nextest"),
        "kani" => tool_in_path("cargo-kani"),
        "miri" => tool_in_path("cargo-miri"),
        "loom" => has_crate_feature("loom", workspace_root),
        "fuzz" => tool_in_path("cargo-fuzz"),
        "mutants" => tool_in_path("cargo-mutants"),
        "coverage" => tool_in_path("cargo-llvm-cov"),
        "verus" => workspace_root.join("verification/verus").exists(),
        "tla" => workspace_root.join("verification/tla").exists(),
        "flux" => tool_in_path("cargo-flux"),
        _ => false,
    }
}

fn tool_in_path(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn has_crate_feature(_feature: &str, _workspace_root: &Path) -> bool {
    // Simplified: check if any crate has the feature
    // In production, parse Cargo.toml for feature flags
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lane_command_test() {
        let lane = Lane {
            name: "test".to_string(),
            required: true,
        };
        let cmd = lane_command(&lane, "vb_core", Path::new("/workspace"));
        assert_eq!(cmd[0], "cargo");
        assert_eq!(cmd[1], "test");
    }

    #[test]
    fn test_lane_command_clippy() {
        let lane = Lane {
            name: "clippy".to_string(),
            required: true,
        };
        let cmd = lane_command(&lane, "vb_core", Path::new("/workspace"));
        assert!(cmd.contains(&"-D".to_string()));
        assert!(cmd.contains(&"warnings".to_string()));
    }

    #[test]
    fn test_required_lanes_always_available() {
        let lanes = detect_available_lanes(Path::new("/workspace"));
        let required: Vec<_> = lanes.iter().filter(|l| l.required).collect();
        assert!(!required.is_empty());
    }
}
