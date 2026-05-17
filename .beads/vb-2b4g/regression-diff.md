# Regression Diff — vb-2b4g

## Scope Baseline

- Delivery scope: `vb_codegen`, including `crates/vb_codegen/src/lib.rs`, `generated_storage_helpers.rs.txt`, and `tests.rs`.
- Required verifier modes: focused `vb_codegen` tests, trybuild, fmt, cargo-check.
- Prior State 11 rejection was only `moon ci` / `lint-src` at `crates/vb_codegen/src/lib.rs:1578` (`clippy::too_many_arguments`).

## Current Classification

- Focused parity/static/compile/fmt/full local `vb_codegen` suite: PASS. PO-007 now records the declared exact command shape via direct cargo binary equivalent: `/home/lewis/.cargo/bin/cargo check -p vb_codegen --all-targets && /home/lewis/.cargo/bin/cargo test -p vb_codegen --test trybuild_tests && /home/lewis/.cargo/bin/cargo fmt --all -- --check`, with cargo check success, 3 trybuild tests passed, and fmt no diff output.
- Prior scoped lint blocker: RESOLVED. `moon ci` output shows `velvet-ballastics:lint-src` completed successfully in 182ms and no `too_many_arguments` diagnostic appeared.
- `moon ci`: DEFERRED_GLOBAL due disk quota/resource exhaustion:
  - `feature-powerset`: failed writing incremental query cache with `Disk quota exceeded`.
  - `fuzz-smoke`: linker/rustc LLVM output stream failed with `Disk quota exceeded`.
  - `mutants-smoke`: failed writing temp mutant file with `Disk quota exceeded`.
  - Moon failed writing `.moon/cache/states/.../stdout.log` with `Disk quota exceeded`.
- Moon-reported `vb_codegen` generated-temp test failures occurred under the same disk-exhausted run; exact focused reruns and full local suite passed, so no bead-local semantic/code failure is established.

## Decision

No remaining scoped `vb_codegen` failure was reproduced by exact obligations. Workspace gate is deferred as environment/global resource debt with exact evidence.
