bead_id: vb-qi37.13.4
bead_title: cli: Structured output contract tests
phase: State 2
updated_at: 2026-05-11T00:00:00Z

# Codebase Map

- CLI crate: `crates/vb_cli`.
- Main binary entrypoint: `crates/vb_cli/src/main.rs`.
- Parser and command enum: `crates/vb_cli/src/args.rs`.
- Existing black-box CLI tests: `crates/vb_cli/tests/`.
- Binary names: `crates/vb_cli/Cargo.toml` defines only `velvet-ballastics` (canonical, no `vb` alias per naming contract).
- Structured output currently uses `--json|--jsonl` via `OutputFormat`; master-level `--emit text|yaml|postcard` structured contract is not implemented in the parser surface seen during State 2.
- Diagnostics are mostly text or JSON `success/error` objects from `json_error`; no uniform diagnostic envelope with `code/path/span/message/repair` was found in the CLI entrypoint.
- `submit` and `simulate` command surfaces already exist in `main.rs`; tests here should lock stdout/stderr separation, exit code behavior, and structured output shape without adding runtime side effects.
