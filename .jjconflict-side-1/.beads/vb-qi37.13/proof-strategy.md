bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 4
updated_at: 2026-05-14T23:05:00Z
attempt: 2-of-7

# Proof Strategy

## Planner Basis

This repair follows proof-planner skill `version=1.0.1`:

- `planner_not_writer`: State 4 writes only planning artifacts under `.beads/vb-qi37.13/`.
- `traceability_required`: every planned obligation maps to a requirement and contract clause.
- `agents_untrusted_tools_decide`: planned rows stay `planned`; existing child evidence is context, not a State 4 pass claim.
- `waivers_are_obligations`: skipped verifier lanes and tooling blockers are represented with rationale and follow-up trigger.

## Scope and Discovery

All discovery was run from `/home/lewis/src/vb-qi37-13-r2` only. The source checkout `/home/lewis/src/Velvet-ballistics` and broken checkout `/home/lewis/src/vb-qi37-13` were not used.

Required inputs were present:

```bash
test -s ".beads/vb-qi37.13/contract.md" && test -s ".beads/vb-qi37.13/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.13/delivery-scope.jsonl"
```

Risk discovery commands:

```bash
rg -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/velvet_ballistics/src/exit_code.rs crates/velvet_ballistics/src/main.rs crates/velvet_ballistics/src/mode_error.rs crates/velvet_ballistics/src/mode_activation_tests.rs crates/velvet_ballistics/src/args.rs crates/velvet_ballistics/src/cli_postcard.rs verification/verus/diagnostic_envelope_verus.rs fuzz/Cargo.toml fuzz/fuzz_targets.rs fuzz/src/lib.rs fuzz/src/bin/vb_ui_model_postcard_decode.rs
rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/velvet_ballistics/src/exit_code.rs crates/velvet_ballistics/src/main.rs crates/velvet_ballistics/src/mode_error.rs crates/velvet_ballistics/src/mode_activation_tests.rs crates/velvet_ballistics/src/args.rs crates/velvet_ballistics/src/cli_postcard.rs verification/verus/diagnostic_envelope_verus.rs fuzz/Cargo.toml fuzz/fuzz_targets.rs fuzz/src/lib.rs fuzz/src/bin/vb_ui_model_postcard_decode.rs
```

Discovery summary:

- Scoped production files use `#![forbid(unsafe_code)]`; `fuzz/fuzz_targets.rs` uses Rust 2024 `#[unsafe(no_mangle)]` export wrappers for libFuzzer targets.
- `verification/verus/diagnostic_envelope_verus.rs` now contains `lemma_exit_code_range_0_to_8` and `spec_exit_code_in_range_0_to_8`.
- `crates/velvet_ballistics/src/cli_postcard.rs` contains postcard decode/encode tests with test-only `unwrap()` calls.
- `crates/velvet_ballistics/src/mode_activation_tests.rs` contains a `proptest!` parser test unrelated to the postcard proof route.
- `fuzz/src/lib.rs` contains postcard schema/kind assertions for the integrated `vb_ui_model_postcard_decode` route.

## Required Lanes

State 4 owns these planned command lanes in `.beads/vb-qi37.13/proof-obligations.planned.jsonl`:

- `VERUS-EXIT-001`: direct Verus proof for public exit-code range `0..=8`.
- `TEST-EXIT-001`: cargo test gate for public exit-code taxonomy and conversion behavior.
- `STATIC-EXIT-001`: static scan for stale public code `9` and stale `0_to_9` proof residue.
- `TEST-DIAGNOSTICS-001`: cargo test gate for unsupported commands/modes failing closed with exit code `1`.
- `TEST-STRUCTURED-001`: cargo test gate for structured-output format parity and machine-readable diagnostics.
- `TEST-POSTCARD-001`: cargo test gate for postcard decode rejection and roundtrip cases.
- `FUZZ-POSTCARD-001`: cargo-fuzz route pinned to `x86_64-unknown-linux-gnu`.
- `RECON-CHILD-001`: exact child evidence marker reconciliation across State 5/6 evidence artifacts.
- `MATRIX-COMMAND-001`: exact command matrix validation for primary obligations and traceability rows.

## Postcard Fuzz Decision

The accepted executable fuzz lane is:

```bash
TMPDIR=/home/lewis/src/vb-qi37-13-r2/target/tmp RUSTC_WRAPPER= cargo fuzz run vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1
```

The default `cargo fuzz run vb_ui_model_postcard_decode -- -runs=1` path is not a required proof command. Existing evidence shows it selects `x86_64-unknown-linux-musl` in this environment and hits an ASAN/static-libc incompatibility. That is a tooling note and waiver candidate only, with follow-up trigger if formal-verifier later requires the default target instead of the pinned GNU target.

## Waiver and Non-Applicable Lanes

- TLA+ is not applicable because this bead is local CLI mapping/codec reconciliation, not temporal lifecycle or protocol behavior.
- Loom is not applicable because scoped production files do not introduce concurrency primitives or scheduling behavior.
- Miri is optional only because scoped production files forbid unsafe code; no Miri command is required for this parent reconciliation bead.
- Broad Kani is not required as a State 4 planned command because the accepted postcard route is unit postcard tests plus integrated cargo-fuzz. If reviewers reject that route, the fallback is a new proof-planner repair, not a silent State 5 requirement.

## Review Focus

Proof reviewer must reject the plan if any primary obligation ID is missing, duplicated, still uses a `PO-*` alias instead of the repaired primary ID, contains a placeholder command, sets `status` to `PASS`, or treats the default musl/ASAN blocker as a required pass command.
