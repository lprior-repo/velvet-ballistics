# Proof Review — vb-om21 State 6

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-om21-state6-001
writer_invocation_id: proof-writer-vb-om21-state5-002
bead_id: vb-om21
state: 6
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-om21
reviewed_at_utc: 2026-05-25T20:20:00Z

## Verdict

REJECTED. State 5 produced smoke-compilable files, not acceptable proof artifacts. Required lanes are either disconnected from their verifier, not discoverable by the exact planned command, or non-vacuous only as ordinary Rust syntax smoke. The proof evidence itself states that formal execution is pending (`proof-evidence.md:38-45`); the cheap smoke evidence does not establish that the verifier lanes are runnable or meaningful.

## Findings First

1. **BLOCKER — VERUS_ARTIFACTS_ARE_NOT_VERUS_PROOFS**
   - Obligations: all 11 `verus` obligations, including `PO-vb-om21-prefix-bound-verus`.
   - Artifact evidence: `verification/verus/vb_om21_tail_fallback_prefix_bound.rs:1-70` contains ordinary Rust functions, no `verus!`, `spec fn`, `proof fn`, `requires`, `ensures`, or verifier contract binding to production `exec` functions. The claimed evidence is only `rustc --crate-type=lib --emit=metadata` in `proof-evidence.md:13`.
   - Required fix: replace the Verus lane with actual Verus artifacts that bind executable production seams or explicitly modeled exec functions using Verus contracts, then record exact `verus --crate-type=lib ...` output and trust-marker scan results.

2. **BLOCKER — FLUX_ARTIFACTS_ARE_NOT_FLUX_REFINEMENTS**
   - Obligations: all 11 `flux-rs` obligations, including `PO-vb-om21-prefix-bound-flux`.
   - Artifact evidence: `verification/flux/vb_om21_tail_fallback_prefix_bound.rs:1-32` contains ordinary Rust with no `flux_rs` attributes, refined types, invariants, `#[sig]`, `#[trusted]` ledger scan, or negative invalid-state rejection target. Evidence again only cites Rust syntax smoke (`proof-evidence.md:13`).
   - Required fix: write real Flux RS refinement artifacts or an approved waiver for Flux non-applicability; run exact `cargo flux -p vb_storage --lib --features flux-proofs -- --check ...` commands and provide solver/tool output plus trust/ignore scan evidence.

3. **BLOCKER — KANI_HARNESSES_NOT_DISCOVERABLE_BY_PLANNED_COMMANDS**
   - Obligations: all 11 `kani` obligations, including `PO-vb-om21-prefix-bound-kani`.
   - Artifact evidence: `crates/vb_storage/src/kani_vb_om21_prefix_bound.rs:18-20` defines a harness, but `crates/vb_storage/src/lib.rs:34-62` registers only pre-existing `kani_*` modules and does not register any `kani_vb_om21_*` modules. State 5 explicitly says no module registration was added (`proof-evidence.md:36`). The planned command `cargo kani -p vb_storage --harness vb_om21_prefix_bound_harness` therefore has no proven harness inventory or discoverability evidence.
   - Required fix: wire each Kani harness into the crate under `#[cfg(kani)]` or provide a valid standalone Kani target, run `cargo kani list --format json`, then run each exact planned harness command with property summary and non-vacuity/assumption scans.

4. **BLOCKER — PROPTEST_FILES_ARE_NOT_CARGO_TEST_TARGETS**
   - Obligations: all 11 `proptest` obligations, including `PO-vb-om21-prefix-bound-proptest`.
   - Artifact evidence: proof artifacts are placed under `crates/vb_storage/tests/proptest/` (for example `crates/vb_storage/tests/proptest/vb_om21_prefix_bound_proptest.rs:1-22`). Cargo auto-discovers integration tests directly under `tests/`; this nested directory has no shown root `tests/proptest.rs` or module registration. No `cargo nextest` command was run (`proof-evidence.md:44`).
   - Required fix: expose these as actual integration tests or crate test modules, then run the exact `cargo nextest run -p vb_storage ...` commands with at least the planned case bound evidence.

5. **BLOCKER — FUZZ_TARGET_NOT_REGISTERED**
   - Obligation: `PO-vb-om21-key-parse-fuzz`.
   - Artifact evidence: `fuzz/fuzz_targets/vb_om21_key_parse_key_parser.rs:1-10` exists, but `fuzz/Cargo.toml:70-160` shows registered fuzz binaries and contains no `vb_om21_key_parse_key_parser` target. A content search for `vb_om21` in `fuzz/Cargo.toml` returned no matches. The exact planned command `cargo +nightly fuzz run vb_om21_key_parse_key_parser -- -runs=100000` is therefore not backed by a registered target.
   - Required fix: register the fuzz target in `fuzz/Cargo.toml` or regenerate it with `cargo fuzz add`, then run the exact planned smoke command and report sanitizer/runtime output.

6. **BLOCKER — TLA_MODELS_ARE_DISCONNECTED_AND_WEAKLY_SPECIFIED**
   - Obligations: all 6 `tla-plus` obligations, including `PO-vb-om21-prefix-bound-tla`.
   - Artifact evidence: `verification/tla/vb_om21_tail_fallback_prefix_bound.tla:10-22` models only `observed \in SUBSET Seqs`; it does not model ordered storage keys, other-run keys, prefix termination, or key bytes despite the prefix-bound claim. `run` is not used by `Classify` or `Next`. `verification/tla/vb_om21_tail_fallback_prefix_bound.cfg:2-5` checks only `TypeInvariant` and `TypedFailureReachable`; the declared `DeadlockFreedom == TRUE` at `.tla:29` is not a meaningful invariant/property.
   - Required fix: model key order, run prefixes, other-run keys, scan termination, and typed outcomes in the TLA state machine; check `TypeOK`, semantic invariants, and explicit deadlock stance with raw TLC output.

## Evidence Reviewed

- `proof-obligations.planned.jsonl`: 52 required obligations.
- `verifier-lane-decisions.jsonl`: required lanes include TLA+, Verus, Kani, Flux, Miri, proptest, and cargo-fuzz.
- `proof-evidence.md`: smoke-only evidence and pending formal execution lines 38-45.
- `trusted-base-ledger.jsonl`: 54 active rows with reviewer disposition still `pending_proof_reviewer`; bounds do not compensate for disconnected or undiscoverable proof artifacts.
- Representative proof artifacts listed in findings above.

## Provenance

- Latest writer invocation in ledger: `proof-writer-vb-om21-state5-002`.
- This review invocation: `proof-reviewer-vb-om21-state6-001`.
- No self-approval detected; reviewer skill differs from writer skill.

STATUS: REJECTED
