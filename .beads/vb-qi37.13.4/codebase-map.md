bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

- CLI crate: `crates/velvet_ballastics`.
- Main binary entrypoint: `crates/velvet_ballastics/src/main.rs`.
- Parser and command enum: `crates/velvet_ballastics/src/args.rs`.
- Existing black-box CLI tests: `crates/velvet_ballastics/tests/cli_integration.rs`.
- Existing alias binaries: `crates/velvet_ballastics/Cargo.toml` defines `velvet-ballastics` and `vb`.
- Structured output currently uses `--json|--jsonl` via `OutputFormat`; master-level `--emit text|yaml|postcard` structured contract is not implemented in the parser surface seen during State 2.
- Diagnostics are mostly text or JSON `success/error` objects from `json_error`; no uniform diagnostic envelope with `code/path/span/message/repair` was found in the CLI entrypoint.
- `submit` and `simulate` command surfaces already exist in `main.rs`; tests here should lock stdout/stderr separation, exit code behavior, and structured output shape without adding runtime side effects.
