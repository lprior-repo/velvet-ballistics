// Citation audit scanner entry.
// Compiles with: rustc --edition=2024 scripts/audit-spec-citations.rs
// Runs with: target/audit-spec-citations <repo-root> [file ...]
//
// Preserves all checks and reporting of scripts/audit-spec-citations.py:
// - Citation extraction from // comment lines
// - Path resolution: registry, explicit, suffix, basename
// - Line range validation
// - Contextual identifier matching
// - Cargo.lock pin verification for registry citations
//
// Exit codes: 0 = pass, 1 = citation failures, 64 = usage error,
//            2 = scanner internal error, 3 = I/O error

const REGISTRY_PREFIX: &str = "/.cargo/registry/src/";

// STOP set from Python source: 58 common terms that are not
// contextual identifiers
const STOP: &[&str] = &[
    "types",
    "rs",
    "crates",
    "vb_runtime",
    "vb_core",
    "vb_storage",
    "vb_ipc",
    "src",
    "shard",
    "production",
    "production_inner",
    "mirror",
    "variant",
    "field",
    "struct",
    "enum",
    "fn",
    "pub",
    "const",
    "the",
    "at",
    "is",
    "in",
    "of",
    "and",
    "per",
    "line",
    "lines",
    "range",
    "run",
    "admission",
    "parts",
    "ipc",
    "runtime",
    "transitions",
    "verus",
    "verification",
    "extern",
    "spec",
    "proof",
    "model",
    "file",
    "see",
    "for",
    "with",
    "from",
    "this",
    "that",
    "into",
    "over",
    "under",
    "when",
    "where",
    "which",
    "also",
];

#[path = "audit_spec_citations/driver.rs"]
mod audit_spec_citations_driver;

fn main() {
    let args = std::env::args_os()
        .skip(1)
        .map(|argument| argument.into_string())
        .collect::<Result<Vec<_>, _>>();
    let exit_code = match args {
        Ok(args) => audit_spec_citations_driver::run(&args),
        Err(_) => write_error("arguments must be valid UTF-8", 64),
    };
    std::process::exit(exit_code);
}

fn write_error(message: &str, fallback: i32) -> i32 {
    use std::io::Write;
    match writeln!(std::io::stderr().lock(), "{message}") {
        Ok(()) => fallback,
        Err(_) => 3,
    }
}
