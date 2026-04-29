# vb-mli Implementation Report

## Files Changed

- `crates/vb-compiler/src/lib.rs`
- `crates/vb-compiler/src/schema.rs`
- `vb-mli/implementation.md`

## Behavior Added

- Repaired the black-hat rejection by preserving the existing compiler diagnostic boundary: `compile` and `parse_ast` now run schema-specific input validation only after the older strict YAML, duplicate-key, structural, and lowering checks succeed.
- Removed the duplicated workflow validator from `schema.rs`; general workflow checks remain in `lib.rs`, while `schema.rs` owns only input-schema-specific validation.
- Kept new validation code mechanical and split oversized functions from the rejected version below the 25-line function rule.
- Added parity tests proving `compile` and `parse_ast` report the same first diagnostic for representative schema failures.
- Added ordering tests proving schema validation does not preempt strict YAML/profile, duplicate-key, last-step, or non-last `finish` diagnostics.

## Command Results

- `rtk cargo fmt --all -- --check`: pass
- `rtk cargo test -p vb-compiler`: pass, 78 tests across 2 suites
- `rtk cargo test --workspace --all-targets`: pass, 165 tests across 15 suites
- `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass, no issues found
