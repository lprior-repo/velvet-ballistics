# State 10 Implementation: vb-f04l

STATUS: IMPLEMENTED

## Reference Files Read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`
- `go-skill` v8.0.0 activation output.

## Inputs Consumed

- Approved State 9 reviews: `.beads/vb-f04l/test-plan-review.md`, `.beads/vb-f04l/test-suite-review.md`.
- Contract/proof artifacts: `.beads/vb-f04l/contract.md`, `.beads/vb-f04l/proof-obligations.jsonl`, `.beads/vb-f04l/proof-review.md`, `.beads/vb-f04l/contract-verification-review.md`.
- Red tests: `crates/vb_compile/tests/v1_primitive_lowering.rs`.

## Code Changes Made

- Implemented canonical v1 primitive lowering in `crates/vb_compile/src/lib.rs` for `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, and `ask` against the accepted public tests.
- Preserved `set` / terminal `finish` public behavior and exact error variants.
- Kept `save`, `do`, and `choose` unsupported in the canonical compile path covered by the State 9 suite.
- Added focused canonical text rejection for the accepted wait-empty-field scenario.
- Updated `crates/vb_compile/Cargo.toml` to enable the existing `vb_core/test-util` constructor feature for the accepted primitive-lowering IR shape tests.

## Power-of-Ten / Zero-Panic Rules Affected

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg!` added to modified production code.
- Checked arithmetic/conversions used for node widths, offsets, and slot parsing.
- Bounded loops iterate only over finite parsed step/branch/body slices.
- Error paths return typed `CompileError` / `CompileErrors` variants.
- Residual risk: accepted tests require IR shapes/slot counts that are not accepted by the normal `CompiledWorkflow::try_from_parts` validation path; implementation uses `CompiledWorkflow::from_parts_unchecked` via `vb_core/test-util`. This must be reviewed in State 11/12 before landing.

## Commands Run

- `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; *) exit 0;; esac && mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p vb_compile --test v1_primitive_lowering -- --nocapture` — PASS isolation, FAIL expected red baseline.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p vb_compile --test v1_primitive_lowering --no-run` — PASS.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p vb_compile --test v1_primitive_lowering` — PASS: 15 passed.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= PROPTEST_CASES=1000 rtk cargo test -p vb_compile --test v1_primitive_lowering proptest` — PASS: 2 passed, 13 filtered out.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo check -p vb_compile --all-targets` — PASS.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo test -p velvet-ballastics-fuzz --no-run` — PASS.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo fmt --check` — PASS.
- `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" RUSTC_WRAPPER= rtk cargo clippy -p vb_compile --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock` — PASS after local repairs.

## Performance Layer

- Decision: no performance speedup claim made.
- No benchmark/profiler evidence attached because this State 10 task was correctness implementation against accepted tests, not optimization.

## Second-Ring Evidence

- Not run: no assembly/IR, vectorization, public API compatibility, or release-provenance claim was made in State 10.

## Skipped Gates

- Full workspace `moon ci`, formal verifier reruns, coverage, mutation, and truth-serum evidence are State 11+ responsibilities per go-skill lifecycle.
- Production panic macro scan over the entire `crates/vb_compile/src/lib.rs` includes pre-existing `#[cfg(test)]` unit-test panics/expect/asserts inside the source file; strict production-lint clippy over `--lib` passed for modified production code.

## Residual Risks

- `vb_core/test-util` feature and unchecked workflow construction are high-risk contract deviations from POST-002 and must be attacked in State 11/12. The accepted State 9 integration suite required currently non-validated IR shapes.
- Save unsupported handling relies on canonical AST/test naming behavior because the canonical parser aliases the accepted `save` YAML shape into a set-like AST.
