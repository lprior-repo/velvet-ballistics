# Proof Review — vb-om21 State 6 Attempt 2

reviewer_skill: proof-reviewer
reviewer_invocation_id: proof-reviewer-vb-om21-state6-002
writer_invocation_id: proof-writer-vb-om21-state5-004
bead_id: vb-om21
state: 6
sublane: proof-review
isolated_workdir: /home/lewis/isolated/femdation-velvet-ballistics/vb-om21
reviewed_at_utc: 2026-05-25T21:35:00Z

## Verdict

REJECTED. The official State 5 PASS covers active artifact hygiene only, not proof validity. The active State 5 report and evidence say the last repair only accounted for a documentation scanner token and made no Verus/Flux/Kani/proptest/fuzz/TLA approval claim (`proof-writer-report.md:9-33`, `proof-evidence.md:9-42`). The archived State 6 rejection remains mathematically unresolved: required lanes are still smoke-shaped, undiscoverable, not registered, or disconnected from the planned verifier commands.

## Findings First

1. **BLOCKER — FORMAL_PROOF_EVIDENCE_ABSENT_AFTER_STATE5_REPAIR**
   - Obligations: all 52 required obligations in `proof-obligations.planned.jsonl`.
   - Artifact evidence: `proof-writer-report.md:9-33` states the repair touched only scanner-token accounting and does not assert verifier approval; `proof-evidence.md:9-42` records only the State 5 hygiene validation file; `state5-trust-marker-repair-validation.json:1-6` reports a PASS without any raw Verus, Flux, Kani, TLC, Miri, proptest, or fuzz verifier output.
   - Required fix: run or explicitly waive every required planned proof obligation with raw command output and non-vacuity evidence, then submit active State 5 proof evidence rather than hygiene-only repair evidence.

2. **BLOCKER — VERUS_ARTIFACTS_ARE_NOT_VERUS_PROOFS**
   - Obligations: all 11 `verus` obligations, including `PO-vb-om21-prefix-bound-verus`.
   - Artifact evidence: `verification/verus/vb_om21_tail_fallback_prefix_bound.rs:1-70` is ordinary Rust. It has no `verus!` block, `spec fn`, `proof fn`, `requires`/`ensures` contract, or binding to production `exec` functions. The active evidence supplies no `verus --crate-type=lib ...` output.
   - Required fix: replace smoke kernels with real Verus artifacts bound to production seams or approved exec models, then record exact Verus command output and marker scan evidence.

3. **BLOCKER — FLUX_ARTIFACTS_ARE_NOT_FLUX_REFINEMENTS**
   - Obligations: all 11 `flux-rs` obligations, including `PO-vb-om21-prefix-bound-flux`.
   - Artifact evidence: `verification/flux/vb_om21_tail_fallback_prefix_bound.rs:1-32` contains ordinary Rust with no `flux_rs` attributes, refined types, `#[sig]`, typestate invariant, negative invalid-state rejection target, or `cargo flux` output.
   - Required fix: encode real Flux RS refinements or obtain an approved waiver; run the exact planned `cargo flux` commands and include solver output plus marker scan evidence.

4. **BLOCKER — KANI_HARNESSES_NOT_DISCOVERABLE_BY_PLANNED_COMMANDS**
   - Obligations: all 11 `kani` obligations, including `PO-vb-om21-prefix-bound-kani`.
   - Artifact evidence: planned harness modules are under `crates/vb_storage/src/kani_vb_om21_*.rs`, but `crates/vb_storage/src/lib.rs:34-62` registers only pre-existing `kani_*` modules and no `kani_vb_om21_*` modules. There is no active `cargo kani list --format json` or exact harness execution output.
   - Required fix: wire every Kani harness into the crate or provide valid standalone targets; run `cargo kani list --format json` and all exact planned harness commands with non-vacuity and assumption evidence.

5. **BLOCKER — PROPTEST_FILES_ARE_NOT_CARGO_TEST_TARGETS**
   - Obligations: all 11 `proptest` obligations, including `PO-vb-om21-prefix-bound-proptest`.
   - Artifact evidence: proptest files are nested under `crates/vb_storage/tests/proptest/`; Cargo auto-discovers integration tests directly under `tests/`, and the `crates/vb_storage/tests` listing shows no root module that registers those nested files. There is no active `cargo nextest run -p vb_storage ...` evidence.
   - Required fix: expose these as actual integration tests or crate test modules, then run exact planned nextest commands with the planned case bounds.

6. **BLOCKER — MIRI_TEST_NOT_DISCOVERABLE_BY_PLANNED_COMMAND**
   - Obligation: `PO-vb-om21-key-parse-miri`.
   - Artifact evidence: `crates/vb_storage/tests/miri/vb_om21_key_parse_miri.rs:1-18` is nested below `tests/miri/` with no shown root integration-test module registration. The planned command `cargo +nightly miri test -p vb_storage vb_om21_key_parse_miri` has no active raw output.
   - Required fix: expose the Miri case as a discoverable test target or registered module, then run the exact planned Miri command and record raw output.

7. **BLOCKER — FUZZ_TARGET_NOT_REGISTERED**
   - Obligation: `PO-vb-om21-key-parse-fuzz`.
   - Artifact evidence: `fuzz/fuzz_targets/vb_om21_key_parse_key_parser.rs` exists, but `fuzz/Cargo.toml:70-169` shows registered fuzz binaries and no `vb_om21_key_parse_key_parser`; content search for `vb_om21` in `fuzz/Cargo.toml` returned no matches. The exact planned fuzz command is not backed by a registered target.
   - Required fix: register the fuzz target in `fuzz/Cargo.toml` or regenerate it with `cargo fuzz add`, then run the exact planned fuzz command and record sanitizer/runtime output.

8. **BLOCKER — TLA_MODELS_DISCONNECTED_FROM_PREFIX_SCAN**
   - Obligations: all 6 `tla-plus` obligations, including `PO-vb-om21-prefix-bound-tla`.
   - Artifact evidence: `verification/tla/vb_om21_tail_fallback_prefix_bound.tla:10-22` models only `observed \in SUBSET Seqs`; it does not model ordered storage keys, other-run keys, byte prefixes, prefix termination, or the scan boundary claimed by the obligation. `run` is unused by `Classify`/`Next`; `verification/tla/vb_om21_tail_fallback_prefix_bound.cfg:2-5` checks only `TypeInvariant` and `TypedFailureReachable`; `DeadlockFreedom == TRUE` at line 29 is not a meaningful checked property.
   - Required fix: model key order, per-run prefix ranges, other-run keys, scan termination, and typed outcomes directly; check TypeOK, semantic invariants, explicit deadlock stance, and raw TLC output.

## Evidence Reviewed

- `proof-obligations.planned.jsonl`: 52 required obligations.
- `verifier-lane-decisions.jsonl`: required lanes include TLA+, Verus, Kani, Flux, Miri, proptest, and cargo-fuzz.
- `proof-writer-report.md`: active State 5 attempt 4 is a scanner-token repair only.
- `proof-evidence.md`: active evidence records hygiene validation, not raw verifier output.
- `state5-trust-marker-repair-validation.json`: official State 5 PASS with no proof findings, limited to validator hygiene.
- `prior-State6-rejection/2026-05-25T202000Z/*`: archived rejection context remains unresolved.
- Representative active verification artifacts named in findings above.

## Provenance

- Latest writer invocation in ledger: `proof-writer-vb-om21-state5-004`.
- This review invocation: `proof-reviewer-vb-om21-state6-002`.
- No self-approval detected; reviewer skill differs from writer skill.

STATUS: REJECTED
