# vb-ju4 Implementation

## Changed Files

- `crates/vb-compiler/src/lib.rs`
- `crates/vb-compiler/src/references.rs`
- `crates/vb-compiler/src/references/tests.rs`
- `vb-ju4/implementation.md`

## Design Decisions

- Added a cold compiler-only reference validation module in `vb-compiler`; no runtime crates were changed.
- Wired reference validation through both `YamlCompiler::compile` and `YamlCompiler::parse_ast` after strict YAML validation, duplicate-key validation, existing lowering validation, input schema validation, and AST parsing.
- Kept lowering-first and schema-first diagnostic ordering by running reference validation only after the existing compiler boundary succeeds.
- Built deterministic declaration tables for `inputs`, `vars`, `secrets`, and step IDs from the parsed AST, relying on upstream duplicate validation instead of emitting misleading duplicate-reference diagnostics.
- Accepted declared `$input.<name>`, `$var.<name>`, `$vars.<name>`, `$secret.<name>`, and `$secrets.<name>` references in cold retained AST values.
- Rejected `$runtime.*`, `$now`, `$random`, and current-phase step references with typed diagnostics because they are not legal in deterministic compiled IR yet.
- Added stable `CompileError` variants for `UnknownReferenceRoot`, `IllegalReference`, and `UnknownReferenceName`.
- Kept functions mechanically small and avoided runtime allocations beyond cold `HashSet` declaration tables and diagnostic strings.

## Command Results

- `rtk cargo fmt --all -- --check`
  - First run failed with formatting diffs in `crates/vb-compiler/src/references.rs`.
  - After `rtk cargo fmt --all`, final result: no output, exit success.
- `rtk cargo test -p vb-compiler`
  - `cargo test: 83 passed (2 suites, 0.00s)`
- `rtk cargo test --workspace --all-targets`
  - `cargo test: 170 passed (15 suites, 0.23s)`
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo clippy: No issues found`

## Repair After Black-Hat Rejection

- Fixed `YamlCompiler::compile` so retained cold surfaces such as `examples` cannot bypass reference validation.
- Added compile-boundary tests proving otherwise-compilable workflows reject unknown and illegal retained references.
- Removed the duplicate-name error path from `ReferenceTables`; duplicate declarations remain owned by the existing structural/schema validation layer.
- Re-ran focused reference tests: `cargo test: 7 passed, 78 filtered out`.
- Re-ran full gates after repair:
  - `rtk cargo test -p vb-compiler` => `cargo test: 85 passed`.
  - `rtk cargo test --workspace --all-targets` => `cargo test: 172 passed`.
  - `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings` => `cargo clippy: No issues found`.

## Notes

- `rtk cargo test --workspace --all-targets` and clippy both briefly waited for Cargo locks because they were run in parallel.
- Existing untracked `vb-*` directories were not modified.
