# Proof Plan Review Input: vb-7m54

## Summary

6 obligations: 5 loom concurrency models (VB-CONC-001..005) + 1 xtask command (VB-CONC-XTASK).

All obligations are HIGH risk and REQUIRED.

## Verification Tool: loom

- **Version**: 0.7.2 (latest on crates.io)
- **Appropriateness**: loom is the correct tool for Rust-level concurrency seams (per master doc line 4964)
- **Alternative**: shuttle (if loom unavailable), but master doc specifies loom

## Open Questions

1. **loom dependency MSRV**: loom 0.7.2 requires Rust 1.56+. Need to verify this doesn't conflict with the pinned nightly.
2. **Model location**: Should loom models live in `vb_runtime/src/models/loom/` or as test files in the respective source directories?
3. **xtask dispatch**: How does the loom subcommand map model names to test functions?

## Pre-conditions for Execution

1. Add `loom = "0.7"` to `vb_runtime/Cargo.toml` under `[dev-dependencies]`
2. Implement `xtask/src/loom.rs` with command dispatch
3. Create loom model files with `#[cfg(loom)]` test functions

## Next Steps

1. proof-writer creates loom models + xtask loom command implementation
2. proof-reviewer reviews artifacts for vacuity and correctness
3. formal-verifier executes `cargo xtask loom --model <name>` for each model
