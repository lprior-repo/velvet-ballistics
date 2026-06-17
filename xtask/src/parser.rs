#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

use std::ffi::OsString;

use crate::command_family::CommandFamily;
use crate::error::XtaskCommandError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XtaskCommand {
    Required(CommandFamily),
    Legacy(&'static str),
    Help,
    Version,
}

struct ParsedCommandName<'a>(&'a str);

pub fn parse_xtask_command(
    args: impl IntoIterator<Item = OsString>,
) -> Result<XtaskCommand, XtaskCommandError> {
    let tokens = collect_args(args);
    let command = top_level_command(&tokens)?;
    classify_top_level_command(command, &tokens)
}

fn collect_args(args: impl IntoIterator<Item = OsString>) -> Vec<String> {
    args.into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn top_level_command(tokens: &[String]) -> Result<ParsedCommandName<'_>, XtaskCommandError> {
    tokens
        .get(1)
        .map(String::as_str)
        .map(ParsedCommandName)
        .ok_or_else(|| XtaskCommandError::MissingRequiredInput {
            command: "xtask".to_string(),
            input: "command".to_string(),
        })
}

fn classify_top_level_command(
    command: ParsedCommandName<'_>,
    tokens: &[String],
) -> Result<XtaskCommand, XtaskCommandError> {
    let command = command.0;
    if command == "--help" || command == "-h" {
        return Ok(XtaskCommand::Help);
    }
    if command == "--version" || command == "-V" {
        return Ok(XtaskCommand::Version);
    }
    if let Some(legacy) = parse_legacy(command) {
        return Ok(XtaskCommand::Legacy(legacy));
    }
    parse_required_command(command, tokens)
}

fn parse_required_command(
    command: &str,
    tokens: &[String],
) -> Result<XtaskCommand, XtaskCommandError> {
    let Some(family) = CommandFamily::parse(command) else {
        return Err(XtaskCommandError::UnknownCommand {
            command: command.to_string(),
        });
    };
    validate_required_options(command, tokens)?;
    Ok(XtaskCommand::Required(family))
}

fn parse_legacy(command: &str) -> Option<&'static str> {
    match command {
        "ui-snapshot" => Some("ui-snapshot"),
        "ui-tokens" => Some("ui-tokens"),
        "ui-overlap-check" => Some("ui-overlap-check"),
        "ai-fast" => Some("ai-fast"),
        "ai-deep" => Some("ai-deep"),
        "ai-release" => Some("ai-release"),
        "forbidden-scan" => Some("forbidden-scan"),
        "benchmark-regression-policy" => Some("benchmark-regression-policy"),
        "proof-plan" => Some("proof-plan"),
        "proof-check" => Some("proof-check"),
        "proof-evidence" => Some("proof-evidence"),
        "proof-drift" => Some("proof-drift"),
        "loom" => Some("loom"),
        "cold-adapter-isolation" => Some("cold-adapter-isolation"),
        "list-crates" => Some("list-crates"),
        "proof" => Some("proof"),
        "contracts" => Some("contracts"),
        _ => None,
    }
}

fn validate_required_options(command: &str, tokens: &[String]) -> Result<(), XtaskCommandError> {
    validate_bead_option(command, tokens)?;
    validate_format_option(command, tokens)
}

fn validate_bead_option(command: &str, tokens: &[String]) -> Result<(), XtaskCommandError> {
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if token == "--bead" {
            let Some(value) = iter.next() else {
                return Err(XtaskCommandError::MissingRequiredInput {
                    command: command.to_string(),
                    input: "bead".to_string(),
                });
            };
            if value.is_empty() {
                return Err(XtaskCommandError::InvalidInput {
                    command: command.to_string(),
                    input: "bead".to_string(),
                    reason: "bead id must not be empty".to_string(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_remains_routed_to_legacy_cli() {
        let parsed = parse_xtask_command(["xtask".into(), "contracts".into(), "--check".into()]);

        assert_eq!(parsed, Ok(XtaskCommand::Legacy("contracts")));
    }

    #[test]
    fn cold_adapter_isolation_remains_routed_to_legacy_cli() {
        let parsed = parse_xtask_command(["xtask".into(), "cold-adapter-isolation".into()]);

        assert_eq!(parsed, Ok(XtaskCommand::Legacy("cold-adapter-isolation")));
    }

    #[test]
    fn unknown_top_level_command_still_fails_closed() {
        let parsed = parse_xtask_command(["xtask".into(), "not-a-command".into()]);

        assert_eq!(
            parsed,
            Err(XtaskCommandError::UnknownCommand {
                command: "not-a-command".to_string()
            })
        );
    }
}

fn validate_format_option(command: &str, tokens: &[String]) -> Result<(), XtaskCommandError> {
    let mut iter = tokens.iter();
    while let Some(token) = iter.next() {
        if token == "--format" {
            let Some(value) = iter.next() else {
                return Err(XtaskCommandError::MissingRequiredInput {
                    command: command.to_string(),
                    input: "format".to_string(),
                });
            };
            if value != "jsonl" {
                return Err(XtaskCommandError::InvalidInput {
                    command: command.to_string(),
                    input: "format".to_string(),
                    reason: format!("unsupported output format: {value}"),
                });
            }
        }
    }
    Ok(())
}
