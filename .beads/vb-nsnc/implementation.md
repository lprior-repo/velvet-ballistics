# Implementation: vb-nsnc

## References read before coding

- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`

## Bead artifacts read

- `.beads/vb-nsnc/codebase-map.md`
- `.beads/vb-nsnc/contract.md`
- `.beads/vb-nsnc/test-plan.md`
- `.beads/vb-nsnc/test-plan-review.md`
- `.beads/vb-nsnc/red-phase.md`

## Files changed

- `crates/vb_validate/src/gates.rs`
- `crates/vb_validate/src/diagnostic.rs`
- `crates/vb_validate/src/diag_codes.rs`
- `crates/vb_validate/src/diag_convert.rs`
- `crates/vb_validate/src/diag_render.rs`
- `.beads/vb-nsnc/implementation.md`

## Implementation summary

- Added cold-path capability schema validation to the live `gates.rs` gate-12 path used by `ValidationPipeline::validate_with_contracts`.
- Enforced `MAX_CAPABILITY_NAME_BYTES = 128`, empty-name rejection, ASCII dotted segment grammar, action-id equality with the enclosing `ActionContract`, and deterministic duplicate `(name, action)` detection within one contract.
- Preserved gate-12 missing-contract precedence, then validates capability schema before orphan-contract checks.
- Replaced capability diagnostic scaffold with stable codes/messages for `E050D..E0511` in production diagnostics and synchronized test-only diagnostic split modules.

## Holzmann constraint evidence

- No runtime hot-path parsing, JSON, YAML, HTTP, admission API change, or `CapabilitySet::grants` semantic change was added.
- Validation loops are bounded by provided action-contract slices and capability-name byte length.
- Duplicate detection uses safe iteration and `take`, not unchecked indexing.
- Modified production code adds no `unsafe {}`, `unwrap()`, `expect()`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` calls.
- Capability-name error formatting is bounded for invalid grammar by the 128-byte maximum; too-long errors report only `len` and `max`.

## Commands run and outcomes

- `cargo nextest run -p vb_validate --test capability_contract_schema` before implementation: failed 18/18 as expected red phase; failures were missing schema validation and diagnostic scaffold code.
- `rtk cargo fmt --check`: passed.
- `cargo nextest run -p vb_validate --test capability_contract_schema`: passed 18/18.
- `PROPTEST_CASES=1000 cargo nextest run -p vb_validate --test capability_contract_schema proptest`: passed 2/2.
- `rtk cargo test -p vb_validate --bench capability_schema --no-run`: passed; benchmark target compiled.
- `rtk cargo test -p vb_validate --test capability_schema_kani --no-run`: passed; Kani harness target compiled.
- `rtk cargo test -p vb_validate --test capability_schema_kani`: passed; 0 runtime tests in the suite.
- `rtk cargo test -p velvet-ballastics-fuzz --features fuzz --bin capability_name_schema --no-run`: blocked by pre-existing `vb_storage` compile errors: `crates/vb_storage/src/batch.rs:188` expected `[u8; 17]`, found `Vec<u8>`; `crates/vb_storage/src/recovery/replay/summary.rs:523` called `.get` on `Option<Taint>`.
- `rtk cargo test -p velvet-ballastics-fuzz --features fuzz --bin capability_contract_schema --no-run`: blocked by the same pre-existing `vb_storage` compile errors.
- `rtk cargo check -p vb_validate --all-targets --all-features`: passed, with pre-existing warnings in `crates/vb_validate/src/type_taint_tests.rs` and `crates/vb_validate/src/gate_08_accessor.rs`.
- `rtk cargo test -p vb_validate --lib`: blocked by unrelated pre-existing gate-08 accessor failures: `gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence` and `gates::tests::gate_08_accepts_valid_accessor` both return `AccessorPathInvalid` where tests expect `Ok(())`.
- `rtk cargo clippy -p vb_validate --all-targets --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use`: blocked by pre-existing `vb_core/src/budget.rs` `clippy::manual_unwrap_or` errors.
- `moon ci`: blocked before build by Moon/Git base lookup: `fatal: ambiguous argument 'main': unknown revision or path not in the working tree`.
- `moon run :ci-source`: blocked; Moon reported no task `:ci-source`.

## Residual risks / blockers

- Full workspace and fuzz gates are blocked by unrelated `vb_storage` compile errors documented above.
- Strict clippy is blocked by unrelated `vb_core/src/budget.rs` lints documented above.
- Canonical `moon ci` is blocked by workspace Git base configuration: missing `main` revision in this worktree context.
- Mutation testing, real Kani proof execution, and full Moon CI were not completed due to the blockers above.
