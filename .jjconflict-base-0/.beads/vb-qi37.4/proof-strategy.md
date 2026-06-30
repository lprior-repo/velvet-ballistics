# Proof Strategy: vb-qi37.4

STATUS: PLANNED_REPAIRED_AFTER_STATE_6_REJECTION

updated_at: 2026-05-15T17:53:20-05:00
planner_role: go-skill State 4 proof-planner
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4`

## Scope

Repair proof planning after State 6 attempt 3 rejection. This plan writes planning artifacts only and makes no production, test, proof/model, dependency, config, source-checkout, or State 5 evidence edits.

## Repaired Contract Inputs

- `contract.md` now binds accepted-artifact admission to exact gate count `15`, true required proof flags, digest equality, exact capability grants, persistence-before-ack, duplicate rejection, and capacity failure diagnostics.
- `tla-spec.md` now assigns persistence-before-ack, live-state-after-persistence, duplicate-run rejection, and QueueFull/capacity failure abstraction to `specs/admission_header_before_ack.tla`.
- `verification-layers.md` now names concrete Verus targets for gate/proof flags, digest binding, and capability matching.
- `proof-obligations.jsonl` and `traceability-matrix.jsonl` now use normalized IDs: `TLA-ACK-001`, `TLA-STATE-002`, `VERUS-CAP-003`, `VERUS-GATE-004`, `VERUS-DIGEST-005`, `KANI-ADMIT-006`, `FUZZ-ARTIFACT-007`, `INT-HEADER-008`, `INT-RECOVERY-009`, `STATIC-NO-YAML-010`, `MUT-ERR-011`, `LOOM-JOURNAL-012`, `GATE-CI-013`, `INT-DUPLICATE-014`, and `INT-CAPACITY-015`.

## Risk Classification

- Temporal lifecycle/persistence risk: required TLA+ for header/admission persistence before acknowledgement, no live state before durable admission, duplicate-run rejection, and capacity/storage failure rejection.
- Rust-local invariant risk: required Verus for runtime gate count/proof flags, digest equality, and exact capability cardinality/matching.
- Bounded state/error risk: required Kani or equivalent `verify-deep` evidence for fail-closed admission outcomes, duplicate, and capacity cases.
- Untrusted input risk: required fuzz or equivalent `verify-deep` evidence for malformed accepted-artifact envelopes.
- Concurrency risk: queued journal surfaces exist; require Loom/Shuttle evidence or reviewer-approved waiver if implementation proves no concurrent queue behavior is in scope.
- Runtime shell/integration risk: require `moon ci`, static scan, mutation/deep verification, and recovery/header integration evidence.

## Ledger Authority Policy

- `.beads/vb-qi37.4/proof-obligations.jsonl` remains the authoritative execution ledger for State 5/6 review. It has 15 rows: 5 direct proof rows plus 10 later-state realization rows.
- `.beads/vb-qi37.4/proof-obligations.planned.jsonl` is a State 4 planning superset. Rows 1-15 intentionally mirror the authoritative execution-ledger IDs. Rows 16-21 are planning-policy decisions only and must not be treated as missing executed obligations.
- No required State 5/6 proof obligation may be `blocked_tooling`. Direct TLC/Verus commands are the accepted State 5 proof policy for `TLA-ACK-001`, `TLA-STATE-002`, `VERUS-CAP-003`, `VERUS-GATE-004`, and `VERUS-DIGEST-005`.
- `moon run :verify-proof` is reclassified as non-required tooling debt (`CANONICAL-PROOF-GATE-016`) because State 4 is forbidden to edit tooling and State 5 already recorded direct TLC/Verus evidence. It expires before final release/closure unless a tooling owner repairs or removes the broken wrapper.
- Later-state deep, integration, lint, mutation, and CI rows keep existing canonical commands and expected evidence. They are planned only and are not State 6 proof-acceptance blockers unless their owner state has executed them.

## Command Boundary Policy

- State 5 direct proof commands: `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`, `verus verification/verus/admission_artifact_model.rs`, and `verus verification/verus/capability_artifact_model.rs`.
- State 8 deferred formal/deep realization commands: `moon run :verify-deep` for Kani/fuzz/Loom-or-waiver evidence.
- State 11 deferred integration/release commands: `moon run :lint-src`, `moon run :verify-deep`, and `moon ci` for static, mutation, integration, and workspace gates.
- Non-required tooling debt command: `moon run :verify-proof`; no PASS is claimed and it is not part of the State 5/6 approval bar after this repair.

## Waivers And Not Applicable Lanes

- Flux is not applicable because the repaired obligations are temporal TLA+ and Verus pure-model invariants, not refinement annotations in production Rust.
- Miri primary proof is waived because scoped discovery found `#![forbid(unsafe_code)]` in touched Rust files and no unsafe/FFI/raw-pointer risk trigger; retain second-ring Miri under `verify-deep` if global gates require it.
- Lean/Aeneas/Hax is waived per `lean-contract.md`; Verus and TLA+ own all identified proof kernels.
- Dependency supply-chain proof is not applicable because this planning refresh does not change dependency files.
- `CANONICAL-PROOF-GATE-016` is waived as a required proof obligation and deferred as tooling debt. Compensating evidence is the direct TLC/Verus rows and raw State 5/6 command evidence. This waiver does not waive final workspace CI or any later owner-state realization row.

## Later-State Classification

- State 5 proof-reviewable now: `TLA-ACK-001`, `TLA-STATE-002`, `VERUS-CAP-003`, `VERUS-GATE-004`, `VERUS-DIGEST-005`.
- State 8 later formal/deep realization: `KANI-ADMIT-006`, `FUZZ-ARTIFACT-007`, `LOOM-JOURNAL-012`.
- State 11 later integration/static/mutation/release realization: `INT-HEADER-008`, `INT-RECOVERY-009`, `STATIC-NO-YAML-010`, `MUT-ERR-011`, `GATE-CI-013`, `INT-DUPLICATE-014`, `INT-CAPACITY-015`.
- State 4 policy rows only: `CANONICAL-PROOF-GATE-016`, `FLUX-NOT-APPLICABLE-017`, `MIRI-WAIVE-018`, `LEAN-WAIVE-019`, `SUPPLY-NOT-APPLICABLE-020`, `PROPTEST-NOT-APPLICABLE-021`.

## Discovery Evidence

- `pwd -P`: exit=0, `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4`.
- `test -s ".beads/vb-qi37.4/contract.md" && test -s ".beads/vb-qi37.4/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.4/delivery-scope.jsonl"`: exit=0.
- `jq -c . ".beads/vb-qi37.4/delivery-scope.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4/traceability-matrix.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4/proof-obligations.jsonl" >/dev/null`: exit=0.
- Scoped risk scan over delivery-scope files: exit=0; found serialization/deserialization, proof flags, runtime state mutation, retry/cancel/state fields, `Mutex`, and queued journal surfaces.
- Scoped verifier scan over delivery-scope plus `specs` and `verification`: exit=0; found repaired Verus proof functions in `verification/verus/admission_artifact_model.rs` and `verification/verus/capability_artifact_model.rs`, plus existing unrelated verification artifacts outside this bead scope.

## Blockers

- No State 4 planning blocker remains.
- `CANONICAL-PROOF-GATE-016` is now non-required tooling debt with direct TLC/Verus compensating evidence. It must not block State 6 approval unless proof review rejects the direct-command evidence policy itself.
- Later owner-state rows remain required for their states and must not be claimed complete in State 6.
