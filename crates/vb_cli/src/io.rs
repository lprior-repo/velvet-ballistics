//! I/O helpers for velvet-ballistics.
#![forbid(unsafe_code)]

use crate::args::ParseError;
use std::io::{self, Write};

pub(crate) const HELP: &str = crate::constants::HELP;
pub(crate) const VERSION: &str = crate::constants::VERSION;

pub(crate) fn write_help_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{HELP}")
}

pub(crate) fn write_version_stdout() -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "velvet-ballistics {VERSION}")
}

pub(crate) fn write_error_stderr(error: &ParseError) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if matches!(error, ParseError::NoCommand) {
        writeln!(handle, "{HELP}")
    } else {
        writeln!(handle, "{error}\n\n{HELP}")
    }
}

pub(crate) fn write_stdout_line(args: std::fmt::Arguments<'_>) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if let Err(error) = handle.write_fmt(args) {
        report_write_failure("stdout write failed", &error);
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        report_write_failure("stdout newline write failed", &error);
    }
}

pub(crate) fn write_stderr_line(args: std::fmt::Arguments<'_>) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(error) = handle.write_fmt(args) {
        report_write_failure("stderr write failed", &error);
        return;
    }
    if let Err(error) = handle.write_all(b"\n") {
        report_write_failure("stderr newline write failed", &error);
    }
}

fn report_write_failure(context: &str, error: &io::Error) {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    if let Err(_fallback_error) = writeln!(handle, "{context}: {error}") {
        // stderr itself failed; no recoverable reporting channel remains.
    }
}

pub(crate) fn exit_from_io(
    result: &io::Result<()>,
    success_code: std::process::ExitCode,
) -> std::process::ExitCode {
    match result {
        Ok(()) => success_code,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}

/// Emit a formatted message to stdout with a trailing newline.
#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        if let Err(_err) = handle.write_fmt(format_args!($($arg)*)) {
            // best-effort
        }
        if let Err(_err) = handle.write_all(b"\n") {
            // best-effort
        }
    }};
}

/// Emit a formatted message to stderr with a trailing newline.
#[macro_export]
macro_rules! errln {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        if let Err(_err) = handle.write_fmt(format_args!($($arg)*)) {
            // best-effort
        }
        if let Err(_err) = handle.write_all(b"\n") {
            // best-effort
        }
    }};
}

/// Emit a JSON report to stdout and return on failure.
#[macro_export]
macro_rules! emit_json_or_return {
    ($value:expr, $format:expr $(,)?) => {{
        match $crate::output::json_out($value, $format) {
            Ok(()) => {}
            Err(error) => return $crate::output::output_error_exit(&error),
        }
    }};
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::absurd_extreme_comparisons,
        clippy::approx_constant,
        clippy::arithmetic_side_effects,
        clippy::as_conversions,
        clippy::assertions_on_constants,
        clippy::bool_assert_comparison,
        clippy::bool_comparison,
        clippy::borrow_deref_ref,
        clippy::cast_abs_to_unsigned,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::clone_on_copy,
        clippy::cloned_ref_to_slice_refs,
        clippy::collapsible_if,
        clippy::collapsible_match,
        clippy::duplicated_attributes,
        clippy::err_expect,
        clippy::expect_fun_call,
        clippy::expect_used,
        clippy::explicit_counter_loop,
        clippy::field_reassign_with_default,
        clippy::filter_map_next,
        clippy::from_iter_instead_of_collect,
        clippy::get_first,
        clippy::if_let_mutex,
        clippy::if_not_else,
        clippy::implicit_clone,
        clippy::implicit_saturating_sub,
        clippy::inconsistent_struct_constructor,
        clippy::indexing_slicing,
        clippy::inefficient_to_string,
        clippy::io_other_error,
        clippy::items_after_test_module,
        clippy::iter_count,
        clippy::iter_filter_is_ok,
        clippy::iter_filter_is_some,
        clippy::iter_not_returning_iterator,
        clippy::iter_over_hash_type,
        clippy::iter_without_into_iter,
        clippy::large_digit_groups,
        clippy::large_futures,
        clippy::large_stack_arrays,
        clippy::large_types_passed_by_value,
        clippy::len_zero,
        clippy::let_and_return,
        clippy::let_underscore_must_use,
        clippy::manual_div_ceil,
        clippy::manual_let_else,
        clippy::manual_map,
        clippy::manual_saturating_arithmetic,
        clippy::manual_strip,
        clippy::manual_unwrap_or,
        clippy::manual_unwrap_or_default,
        clippy::map_clone,
        clippy::map_flatten,
        clippy::match_like_matches_macro,
        clippy::misnamed_getters,
        clippy::missing_safety_doc,
        clippy::module_inception,
        clippy::mutable_key_type,
        clippy::needless_bool,
        clippy::needless_bool_assign,
        clippy::needless_borrow,
        clippy::needless_borrows_for_generic_args,
        clippy::needless_collect,
        clippy::needless_pass_by_value,
        clippy::needless_range_loop,
        clippy::needless_return,
        clippy::needless_update,
        clippy::neg_cmp_op_on_partial_ord,
        clippy::new_without_default,
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
        clippy::unnecessary_map_or,
        clippy::unnecessary_mut_passed,
        clippy::unnecessary_sort_by,
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
        unused_variables
    )]

    use super::*;

    #[test]
    fn exit_from_io_returns_success_code_on_ok() {
        let result: io::Result<()> = Ok(());
        let code = exit_from_io(&result, std::process::ExitCode::SUCCESS);
        assert_eq!(code, std::process::ExitCode::SUCCESS);
    }

    #[test]
    fn exit_from_io_returns_failure_code_on_err() {
        let result: io::Result<()> = Err(io::Error::new(io::ErrorKind::Other, "test"));
        let code = exit_from_io(&result, std::process::ExitCode::SUCCESS);
        assert_eq!(code, std::process::ExitCode::FAILURE);
    }

    #[test]
    fn exit_from_io_respects_custom_success_code() {
        let result: io::Result<()> = Ok(());
        let custom = std::process::ExitCode::from(42);
        let code = exit_from_io(&result, custom);
        assert_eq!(code, custom);
    }

    #[test]
    fn write_version_stdout_succeeds() {
        let result = write_version_stdout();
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_help_stdout_succeeds() {
        let result = write_help_stdout();
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_error_stderr_formats_missing_argument() {
        let err = ParseError::MissingArgument("test");
        let result = write_error_stderr(&err);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_error_stderr_formats_unknown_emit_target() {
        let err = ParseError::UnknownEmitTarget("json".into());
        let result = write_error_stderr(&err);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_error_stderr_formats_unknown_durability() {
        let err = ParseError::UnknownDurability("fast".into());
        let result = write_error_stderr(&err);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_error_stderr_formats_unknown_command() {
        let err = ParseError::UnknownCommand("foo".into());
        let result = write_error_stderr(&err);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_error_stderr_formats_no_command() {
        let err = ParseError::NoCommand;
        let result = write_error_stderr(&err);
        assert!(matches!(result, Ok(())));
    }

    #[test]
    fn write_stdout_line_does_not_panic() {
        write_stdout_line(format_args!("test message: {}", 42));
    }

    #[test]
    fn write_stderr_line_does_not_panic() {
        write_stderr_line(format_args!("error message: {}", 99));
    }
}
