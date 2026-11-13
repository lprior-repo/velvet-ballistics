//! Error exhaustiveness checker: compares fuzz oracle coverage against
//! production error enum variant definitions.
//!
//! Replaces the embedded Python in scripts/check-error-exhaustiveness.sh.
//! Uses regex 1.12.3 (approved dependency) for pattern matching.

use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[path = "../source_validators/exhaustiveness/parser.rs"]
mod exhaustiveness_parser;

#[path = "../source_validators/exhaustiveness/checks.rs"]
mod checks;
use checks::{CHECKS, Check, Oracle, join_path};

#[path = "../source_validators/exhaustiveness/io_helpers.rs"]
mod exhaustiveness_io;
use exhaustiveness_io::{read_and_check_wildcard, read_and_parse_enum, read_and_parse_oracle};

fn main() -> Result<(), String> {
    let result = run();
    match result {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(message) => {
            let stderr = io::stderr();
            let mut err = stderr.lock();
            err.write_fmt(format_args!("{}\n", message))
                .map_err(|e| format!("stderr write error: {}", e))?;
            std::process::exit(1);
        }
    }
    Ok(())
}

fn parse_root_argument() -> Result<PathBuf, String> {
    let root = env::args_os()
        .nth(1)
        .ok_or_else(|| "Missing required root path argument".to_string())?;
    Ok(PathBuf::from(root))
}

fn run() -> Result<i32, String> {
    let root = parse_root_argument()?;
    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "=== Checking fuzz harness error exhaustiveness ===")
        .map_err(|e| format!("write error: {}", e))?;

    let checks = CHECKS;
    let mut failures = Vec::new();

    checks
        .iter()
        .try_for_each(|check| check_type(&root, check, &mut out, &mut failures))?;

    report_results(&mut out, &failures)?;
    if failures.is_empty() { Ok(0) } else { Ok(1) }
}

fn check_type(
    root: &Path,
    check: &Check,
    out: &mut impl Write,
    failures: &mut Vec<String>,
) -> Result<(), String> {
    let enum_path = join_path(root, check.enum_path);
    let variants = match read_and_parse_enum(&enum_path, check.type_name) {
        Ok(v) => v,
        Err(e) => {
            failures.push(format!(
                "{} enum parse failed in {}: {}",
                check.type_name, check.enum_path, e
            ));
            return Ok(());
        }
    };

    let mut ctx = CheckCtx { out, failures };
    check
        .oracles
        .iter()
        .try_for_each(|oracle| check_oracle(root, check, oracle, &variants, &mut ctx))
}

struct CheckCtx<'a> {
    out: &'a mut dyn Write,
    failures: &'a mut Vec<String>,
}

fn check_oracle(
    root: &Path,
    check: &Check,
    oracle: &Oracle,
    variants: &[String],
    ctx: &mut CheckCtx<'_>,
) -> Result<(), String> {
    let oracle_path = join_path(root, oracle.path);
    let mentions = match read_and_parse_oracle(&oracle_path, check.type_name, oracle.function) {
        Ok(m) => m,
        Err(e) => {
            ctx.failures.push(format!(
                "{} oracle parse failed in {}::{}: {}",
                check.type_name, oracle.path, oracle.function, e
            ));
            return Ok(());
        }
    };

    check_oracle_wildcard(root, check, oracle, ctx);
    check_coverage(check, oracle, variants, &mentions, ctx)
}

fn check_oracle_wildcard(root: &Path, check: &Check, oracle: &Oracle, ctx: &mut CheckCtx<'_>) {
    let oracle_path = join_path(root, oracle.path);
    let wildcard = match read_and_check_wildcard(&oracle_path, oracle.function) {
        Ok(w) => w,
        Err(e) => {
            ctx.failures.push(format!(
                "{} wildcard parse failed in {}::{}: {}",
                check.type_name, oracle.path, oracle.function, e
            ));
            return;
        }
    };
    if wildcard {
        ctx.failures.push(format!(
            "{} graceful wildcard arm accepts an unlisted variant in {}::{}",
            check.type_name, oracle.path, oracle.function
        ));
    }
}

fn check_coverage(
    check: &Check,
    oracle: &Oracle,
    variants: &[String],
    mentions: &[String],
    ctx: &mut CheckCtx<'_>,
) -> Result<(), String> {
    let missing = find_missing_variants(variants, mentions);
    if !missing.is_empty() {
        report_missing(check, oracle, &missing, ctx);
        Ok(())
    } else {
        report_success(check, oracle, variants.len(), &mut ctx.out)
    }
}

fn find_missing_variants<'a>(variants: &'a [String], mentions: &[String]) -> Vec<&'a String> {
    variants.iter().filter(|v| !mentions.contains(v)).collect()
}

fn report_missing(check: &Check, oracle: &Oracle, missing: &[&String], ctx: &mut CheckCtx<'_>) {
    let mut sorted_missing = missing.to_vec();
    sorted_missing.sort();
    let missing_str = sorted_missing
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    ctx.failures.push(format!(
        "{} missing in {}::{}: {}",
        check.type_name, oracle.path, oracle.function, missing_str
    ));
}

fn report_success(
    check: &Check,
    oracle: &Oracle,
    variant_count: usize,
    out: &mut impl Write,
) -> Result<(), String> {
    writeln!(
        out,
        "  OK {} in {}::{}: {} variants",
        check.type_name, oracle.path, oracle.function, variant_count
    )
    .map_err(|e| format!("write error: {}", e))
}

fn report_results(out: &mut impl Write, failures: &[String]) -> Result<(), String> {
    if failures.is_empty() {
        writeln!(out, "=== All error exhaustiveness checks passed ===")
            .map_err(|e| format!("write error: {}", e))?;
    } else {
        let stderr = io::stderr();
        let mut err = stderr.lock();
        writeln!(err, "=== Error exhaustiveness failures ===")
            .map_err(|e| format!("write error: {}", e))?;
        failures.iter().try_for_each(|failure| {
            writeln!(err, "  {}", failure).map_err(|e| format!("write error: {}", e))
        })?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../source_validators/exhaustiveness/write_error_tests.rs"]
mod bin_write_tests;
