# Verification Layers: vb-core-cli-accepted-path

## Boundary

- Verus-owned kernel: digest-binding predicates, strict policy witness typing, admission decision totality.
- TLA+ temporal model: accepted-run ordering, atomic acknowledgement boundary, rejection liveness, no raw strict bypass.
- Theorem projection: none unless accepted artifact format introduces a proof lattice beyond Verus.
- Runtime shell: CLI commands, Fjall persistence, runtime constructor plumbing, shard admission, and operator output.
- External systems excluded from formal proof: OS filesystem, Fjall internals, wall clock, and terminal rendering.

## Layer Assignment

- PRE-001 -> integration + proptest parser/compile fixture coverage; source parser/compiler correctness remains owned by dependencies.
- PRE-002 -> integration + static-scan + API review: strict/journaled CLI constructs storage-backed runtime admission.
- PRE-003 -> integration + storage acceptance tests + Kani/Verus where pure envelope validation exists.
- PRE-004 -> Verus digest-binding model + integration tests.
- PRE-005 -> Verus policy witness model + integration bypass rejection.
- POST-001 -> integration + TLA+ ordering.
- POST-002 -> TLA+ atomic boundary + storage failure-injection integration.
- POST-003 -> integration + static-scan rejecting production `AlwaysPresentArtifactStore` strict construction.
- POST-004 -> TLA+ reject-before-ack + admission unit/integration tests + mutation.
- POST-005 -> integration ensuring relaxed-only behavior remains explicitly non-strict.
- POST-006 -> manual-qa/evidence inspection.
- INV-001 -> TLA+ + integration.
- INV-002 -> Verus + proptest + integration.
- INV-003 -> Verus + static-scan + integration.
- INV-004 -> static-scan + integration.
- INV-005 -> TLA+ + storage failure-injection integration.
- INV-006 -> static-scan + integration/recovery evidence.
- INV-007 -> static-scan + `moon ci` source lint gate.
- ERR-001..ERR-008 -> integration + mutation + typed error assertions.

## Verus Scope

- Digest binding target: `verification/verus/accepted_cli_digest_binding.rs`.
  - Spec functions: `digest_binding_total`, `digest_mismatch_rejects`.
  - Proof functions: `proof_total_binding_implies_all_equal`, `proof_any_pairwise_mismatch_rejects`, `proof_admitted_digest_matches_run_header`.
  - Evidence command: `verus verification/verus/accepted_cli_digest_binding.rs`.
- Strict policy witness target: `verification/verus/strict_admission_witness.rs`.
  - Spec functions: `strict_like`, `storage_backed`, `valid_admission_witness`.
  - Proof functions: `proof_strict_requires_storage`, `proof_journaled_requires_storage`, `proof_raw_parts_not_strict_witness`, `proof_raw_compiled_not_strict_witness`, `proof_always_present_not_strict_witness`, `proof_storage_artifact_satisfies_strict_witness`.
  - Evidence command: `verus verification/verus/strict_admission_witness.rs`.
- Admission decision target: `verification/verus/accepted_artifact_admission_decision.rs`.
  - Required strengthened model functions: `admission_outcome`, `outcome_error`, `outcome_admitted`, `outcome_acknowledged`, `outcome_run_state_inserted`.
  - Required proof functions: one proof per invalid artifact case establishing exact typed error, `admitted=false`, `acknowledged=false`, and `run_state_inserted=false`; one valid-case proof establishing admission is possible only for `Valid`.
  - Evidence command: `verus verification/verus/accepted_artifact_admission_decision.rs`.
- Invariants: digest equality across artifact/header/event/admission; strict policy requires accepted artifact witness; validation decision total over missing/malformed/invalid/mismatch cases; invalid artifacts reject before acknowledgement/admission/state insertion.
- Trusted boundary: postcard decode, Fjall reads/writes, blake3 digest implementation, CLI file input, and runtime shell calls.
- Shell exclusions: I/O, storage engine internals, YAML parser implementation, CLI argument parsing, wall-clock time.

## TLA+ Scope

- Module/model path: `verification/tla/AcceptedCliAdmission.tla` with config `verification/tla/AcceptedCliAdmission.cfg`.
- Variables: policy, source/artifact/header/event persistence sets, admitted/rejected sets, digest relation, validity flags.
- Actions: parse, compile, persist source, persist accepted artifact, persist header/event atomically, admit, reject, fail storage, relaxed run.
- Safety invariants: strict admitted has artifact; accepted event has full boundary; digest binding total; no raw strict bypass; reject before run insertion.
- Temporal properties: eventually accepted/admitted or rejected under fair enabled storage/admission; failure eventually rejected.
- Fairness/deadlock stance: weak fairness on enabled persist/admit/reject; TLC no deadlock except terminal completion.
- Refinement boundary: CLI/runtime/storage event observations map to TLA actions by run id and digest.
- Evidence command: `tlc -config verification/tla/AcceptedCliAdmission.cfg verification/tla/AcceptedCliAdmission.tla`.
- Required repair from State 6: the config must include `PROPERTY` entries for `EventuallyAcceptedOrRejected` and `FailureEventuallyRejected`, or a reviewer-approved terminal-only deadlock/liveness waiver. Safety-only TLC output is insufficient.

## Defense-In-Depth Gates

- Canonical CI: `moon ci`.
- JSONL contract validity: `python3 -m json.tool` per line or equivalent JSONL validation.
- Parser/codec fuzzing: required if accepted artifact decode or strict direct compiled input parsing changes; current executable targets are `rustup run nightly-2026-04-28 cargo fuzz run admission_fuzz -- -runs=1000` and `rustup run nightly-2026-04-28 cargo fuzz run admission_flow -- -runs=1000`.
- Miri: required for touched pure/storage codec paths if existing Moon lane supports it; otherwise formal-verifier records DEFERRED_GLOBAL for missing lane.
- Mutation: required over typed admission errors and bypass rejection tests if cargo-mutants scope exists; bounded smoke evidence command is `moon run :mutants-smoke`, and deeper mutation scope must be planned by State 9/11 if touched code is outside the smoke slice.
- API compatibility: required if public runtime constructors or CLI-facing APIs change; evidence command is `rustup run nightly-2026-04-28 cargo semver-checks check-release --package vb_runtime --baseline-rev origin/main && rustup run nightly-2026-04-28 cargo semver-checks check-release --package velvet_ballastics --baseline-rev origin/main`.
- Release provenance: release-critical bead requires supply-chain/release gate through `moon ci` or release gauntlet used by repository.

## State 3 to State 4/5 Proof ID Mapping

- `TLA-ACCEPT-001` -> `PO-001`.
- `VERUS-DIGEST-001` -> `PO-002`.
- `VERUS-POLICY-001` -> `PO-003`.
- `VERUS-ADMISSION-001` -> `PO-004`.
- `KANI-ADMISSION-001` -> `PO-007`.

## Waivers

- THM-WAIVE-001: no Lean/Aeneas/Hax kernel at contract time; Verus and TLA+ own known proof properties. Revisit after accepted artifact format closes.

## Review Requirement

An independent `contract-verification-review.md` with `STATUS: APPROVED` is required before State 4 proof planning, tests, or implementation consume these artifacts.
