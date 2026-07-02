# Wave 4 — Agent 13: Ad-Hoc Verus-Binding Deep Dive

Scope: `verification/verus/` artifacts, GOD RULE 2 (no vacuum proofs).
Method: per-bug bd show, locate Verus artifact, inspect production-binding (requires/ensures on production exec fn vs spec-only mirror), run `scripts/verify-verus.sh`.

## Bug-by-bug audit

| bug-id | pri | verus-artifact | production-binding | vacuum | verus-cmd | verus-result | verdict | evidence |
|--------|-----|----------------|--------------------|--------|-----------|--------------|---------|----------|
| vb-vbdco | P3 | NONE | NO Verus proof exists for EvidenceCapacityExceeded. Production fix lives in `crates/vb_runtime/src/engine/types.rs:91-180` (push_step_started/succeeded/slot_written return `Result<(), EngineError>`). Grep across `verification/verus/` for `EvidenceCapacityExceeded`/`evidence_capacity` returns zero matches. Bug is closed via production fix + drive.rs propagation + diagnostic 0x140E + regression tests, but the fix has **zero Verus coverage**, so the GOD RULE 2 audit is not applicable (no proof to be vacuum). | N/A (no proof) | n/a | n/a | PATCHED (production fix committed, no Verus obligation on the bead) | `bd show vb-vbdco` (d8221505b + 3bbfa264d + cd2de4c41); `.evidence/vb-vbdco/`; `crates/vb_runtime/src/engine/types.rs:94,112,135` |
| vb-vuebt | P0 | NONE | Bug is purely a test-source fix (delete duplicate `-> Result<...>` clauses in `crates/vb_runtime/src/shard/impl_parts/timer_methods.rs:141`, `chunk_003.rs:88`, `chunk_013.rs:89`). No Verus proof obligation touched. Verus is not in scope. | N/A (no proof) | n/a | n/a | PATCHED | `bd show vb-vuebt`; `.evidence/wave-1-verification/REPORT.md` NEW-2/3/4 |
| vb-w2wde | P0 | `verification/verus/vb_jpq724_events_for_run_production.rs` (header: "Production-bound Verus contracts for vb_storage journal replay seams ... events_for_run seam contracts") | PARTIAL — file claims production binding via documented refinement map (`snapshot_authority_result -> trimming::latest_durable_snapshot_seq + codec::next_seq in journal/replay.rs`), but the *binding is comment-only*. The artifact defines mirror types `SpecEventSeq`, `SpecRunId`, `SpecJournalEvent`, `SpecJournalErrorKind`, `SpecSnapshotStatus` (lines 34-142) and `proof fn`s over them. There is no `exec fn` with `requires`/`ensures`, no `use vb_storage::...`, no `extern_spec`, and grep confirms `spec_events_for_run*` appears only inside the verification file (96 matches, all in `verification/verus/vb_jpq724_events_for_run_production.rs`). Per the AGENTS.md proof-review.md (lines 367-374): "12 of 14 files are completely disconnected from production Rust. They define their own `Spec*` types ... without `use` statements importing production crates, `extern_spec` or `#[verifier::external]` bindings, `BINDING` comments mapping spec types to Rust types, or executable wrappers with `ensures` clauses." Production fix itself (MAX_INITIAL_REPLAY_CAPACITY=4096, `Vec::with_capacity(limit.max_events().min(MAX_INITIAL_REPLAY_CAPACITY))` at `crates/vb_storage/src/journal/replay.rs:14-23,152-158`) is correct Power-of-Ten, but the Verus proof does NOT mathematically bind to it — the proof only proves properties of the spec-mirror universe. The capacity-overflow bug fix is therefore NOT proven by this Verus artifact. | YES (vacuum) | `verus --crate-type=lib verification/verus/vb_jpq724_events_for_run_production.rs` | `verification results:: 5 verified, 0 errors` (PASS in registry, 5/5) | PARTIAL — production fix correct, Verus file passes type-check, but the artifact is a spec-mirror model, not a requires/ensures binding to the actual `FjallJournal::events_for_run` / `events_for_run_from`. | `.evidence/verus/vb_jpq724_events_for_run_production.txt` (3.2K); `verification/verus/vb_jpq724_events_for_run_production.rs:1-15,191-228`; contracts/proof_obligations.yaml:493 |
| vb-wb05o | P3 | `verification/verus/capability_artifact_model.rs` AND `verification/verus/accepted_artifact_admission_decision.rs` (both labelled "BINDING: ... Rust type: vb_core::capability::Capability / vb_runtime::admission::ArtifactEnvelopeError") | VACUUM — both files explicitly self-declare as pure models. `capability_artifact_model.rs:11-18` declares: "This is a pure model. Fjall I/O, postcard bytes, and production Rust structs remain trusted shell boundaries ... Divergences: Spec models abstract int for name/action; Rust uses Box<str> and ActionId types". The `admit_artifact_run_with_certificate_floor` function (production: `crates/vb_runtime/src/admission.rs:676`) is not imported, not annotated, no `extern_spec`. The mirror types are `int` for `Capability.name` / `ActionId`, but production uses `Box<str>` / `ActionId` (different concrete types), so the spec even has type-divergence from production. `accepted_artifact_admission_decision.rs:1-9` declares: "Divergences: Spec models simplified error variants; Rust has 11 payload-carrying variants" — explicit acknowledgment that the proof does not cover the actual production error set. Neither file is in the verus registry (`contracts/proof_obligations.yaml`), so `verify-verus.sh` does not run them. Bug is duplicate of vb-12yr3 and closed without a Verus patch; the bead's existence demonstrates the production-runtime mismatch fix was NOT accompanied by a Verus-binding patch. | YES (vacuum) | Not in registry → `verify-verus.sh` does NOT execute these files | Not run by registry | NOT-PATCHED — vacuum proofs present but never tied to the `admit_artifact_run_with_certificate_floor` short-circuit fix via requires/ensures on the production exec fn. | `verification/verus/capability_artifact_model.rs:11-18`; `verification/verus/accepted_artifact_admission_decision.rs:1-9`; `crates/vb_runtime/src/admission.rs:662,676` |
| vb-xezc0 | P0 | NONE | Bug is workspace-crate rename (`vb_workspace_tests` everywhere; 14+ test files had stale `velvet_ballistics_workspace_tests::*` imports). Pure build-system fix, no Verus artifact in scope. Verus registry unaffected (still references the same paths). | N/A (no proof) | n/a | n/a | PATCHED | `bd show vb-xezc0`; `crates/workspace_tests/Cargo.toml`; `bd show vb-q37xm` parent |
| vb-xvpec | P0 | NONE | Bug is `kani::assume(false)` gating in `crates/*/src/...` files. The fix is feature-flag/`#[cfg(kani)]` isolation, which is a Kani-only concern. `verification/verus/` has no kani::assume bindings; Verus is not in scope for this bead. | N/A (no proof) | n/a | n/a | PATCHED — bead close-reason confirms `cargo check --workspace --all-targets --all-features` produces 0 kani::assume errors. | `bd show vb-xvpec`; bead close-reason text |

## Verifier run summary

`bash scripts/verify-verus.sh 2>&1 | tail -30` returned:

```
verification results:: 5 verified, 0 errors
VERUS_REGISTRY_OK evidence=.evidence/verus
```

Registry contents (`contracts/proof_obligations.yaml`): 5 distinct Verus targets — `taint_lattice.rs`, `step_state_machine.rs`, `step_budget.rs`, `resource_budget.rs`, `vb_jpq724_events_for_run_production.rs`. All 5 PASS type-check. Trust-scan reports no `assume(...)` / `verifier::external_body` / `verifier::external` / `axiom` matches.

```
PASS verus verification/verus/taint_lattice.rs
PASS verus verification/verus/step_state_machine.rs
PASS verus verification/verus/step_budget.rs
PASS verus verification/verus/resource_budget.rs
PASS verus verification/verus/vb_jpq724_events_for_run_production.rs
VERUS_TRUST_SCAN_OK
```

## Tally

- bugs-checked: 6
- PATCHED: 4 (vb-vbdco, vb-vuebt, vb-xezc0, vb-xvpec — production-fix or test-fix bugs with no Verus obligation on the bead)
- PARTIAL: 1 (vb-w2wde — production fix correct, Verus file passes type-check, but the artifact is spec-mirror only without mathematical binding to production exec fn via requires/ensures)
- NOT-PATCHED: 1 (vb-wb05o — vacuum proofs present, not in registry, no requires/ensures binding to `admit_artifact_run_with_certificate_floor`)
- UNKNOWN: 0

Vacuum proofs in scope (5 files inspected, all in `verification/verus/`):
- `verification/verus/vb_jpq724_events_for_run_production.rs` — mirror types only, refinement map is comments
- `verification/verus/capability_artifact_model.rs` — explicit "pure model" / "trusted shell boundary" declaration, divergent types
- `verification/verus/accepted_artifact_admission_decision.rs` — explicit "spec models simplified error variants" declaration
- `verification/verus/accepted_run_atomic_admission.rs` — explicit "trusted shell boundaries that require later integration"
- `verification/verus/step_state_machine.rs` — has `exec fn validate_transition_exec` with `ensures` but operates on `SpecStepState` mirror, not production `StepState`
- `verification/verus/storage_kind_family.rs` — declares "GOD RULE 2: Verus specs bind to actual Rust implementation behavior" in header but no exec fn / extern_spec binding
- `verification/verus/error_parity.rs` — same pattern, header asserts GOD RULE 2 but no binding
- `verification/verus/diagnostic_envelope_verus.rs` — mirror `SpecCliExitCode`, no production binding
- `verification/verus/ipc_capacity_bounds.rs` — header declares "Production linkage remains REFINE-IPC-002" (acknowledging no binding)
- `verification/verus/taint_lattice.rs` — mirror `SpecTaint` enum, no `use vb_core::value::Taint`

Per `verification/verus/proof-review.md` lines 365-374 the prior reviewer already flagged that **12 of 14 audited files are completely disconnected from production Rust** with no `use` imports, no `extern_spec`, no `BINDING` comments, and no executable wrappers with `ensures`. The five registry-passing files (`taint_lattice`, `step_state_machine`, `step_budget`, `resource_budget`, `vb_jpq724_events_for_run_production`) are precisely the files NOT covered in that earlier 14-file audit — yet the same vacuum pattern is visible in all of them on inspection.

Vacuum-proof count in scope of this chunk: **5 distinct bugs × inspected Verus files = 6 vacuum-grade artifacts** (capability_artifact_model, accepted_artifact_admission_decision, vb_jpq724_events_for_run_production, accepted_run_atomic_admission, step_state_machine, storage_kind_family). Of these, only `vb_jpq724_events_for_run_production.rs` is in the active registry; the others are not registry-required.

## Top-3 NOT-PATCHED (with reason)

1. **vb-wb05o** — `verification/verus/capability_artifact_model.rs` declares itself a "pure model" with `int`-abstracted name/action against production `Box<str>`/`ActionId` (explicit type divergence). No `use vb_runtime::admission`, no `extern_spec`, no `requires`/`ensures` on `crates/vb_runtime/src/admission.rs:676 admit_artifact_run_with_certificate_floor`. Bead is closed as duplicate of vb-12yr3 without a Verus-binding patch. The RA-023 short-circuit fix in production has zero mathematical guarantee from this Verus artifact.

2. **vb-wb05o** — `verification/verus/accepted_artifact_admission_decision.rs` models `ArtifactEnvelopeError` with only 5 spec variants against 11 payload-carrying production variants (`crates/vb_runtime/src/admission::ArtifactEnvelopeError`). The proof is `proof fn ... ensures ... { }` (empty body), proving `outcome_admitted(case) || outcome_rejects(case)` over a 7-variant mirror enum that doesn't match the production shape. Not in registry → not even running through `verify-verus.sh`.

3. **vb-w2wde** — `verification/verus/vb_jpq724_events_for_run_production.rs` defines mirror types `SpecEventSeq`/`SpecRunId`/`SpecJournalEvent` and proves `spec_events_for_run_from_contract`. The `Vec::with_capacity(limit.max_events().min(MAX_INITIAL_REPLAY_CAPACITY))` overflow fix at `crates/vb_storage/src/journal/replay.rs:152-158` is the production patch, but the Verus file contains no `extern_spec` binding to `FjallJournal::events_for_run` / `events_for_run_from`, no `exec fn` with `requires`/`ensures` on the production code, and the "Refinement map" lives only in comment lines 187-190. The file passes Verus type-check (5 verified, 0 errors) precisely because it proves a tautological property of its own mirror universe — not because the production capacity-overflow path is verified.

File written: `/home/lewis/src/velvet-ballistics/to-fix/wave4/agent-13-adhoc-verus-binding.md`
