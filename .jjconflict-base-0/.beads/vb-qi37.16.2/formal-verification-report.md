# Formal Verification Report

bead_id: vb-qi37.16.2
phase: state-12
updated_at: 2026-05-11T23:10:00Z

STATUS: APPROVED

## Inputs

- proof-obligations.jsonl: 13 obligations loaded; JSONL valid after State 12 Verus command repair.
- contract-verification-review.md: approved after State 12 command repair.
- verification-layers.md: updated Verus evidence command to the dedicated harness.
- verus-report.md: approved with executed verifier evidence.
- formal-waivers.jsonl: absent.

## Tool Availability

- `command -v verus` — `/home/lewis/.local/bin/verus`.
- `verus --version` — `0.2026.05.05.d03e906`, toolchain `1.95.0-x86_64-unknown-linux-gnu`.
- `verusfmt` — missing; recorded as `VERUSFMT_MISSING`, not proof evidence.

## Obligation Results

- `TLA-RESUME-001`..`TLA-RESUME-004` — PASS; preserved `tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla` evidence: no errors, 850 generated, 313 distinct, depth 13.
- `VERUS-INV-001` — PASS; `verus .beads/vb-qi37.16.2/verus_resume_harness.rs` exits 0 and verifies `proof_handle_resume_preserves_invariants`.
- `VERUS-PRE-002` — PASS; same harness verifies `proof_is_resumable_exhaustive`.
- `VERUS-PRE-003` — PASS; same harness verifies `proof_hydration_completeness`.
- `VERUS-POST-004` — PASS; same harness verifies `proof_append_immutable`.
- `VERUS-INV-003` — PASS; same harness verifies `proof_resume_result_fields_present`.
- `INTEGRATION-REPLAY-001` — PASS; preserved replay evidence: 3 passed, 0 failed.
- `INTEGRATION-CLI-001` — PASS; preserved evidence: command exited 0, 0 passed, 74 filtered out.
- `UNIT-LIFECYCLE-001` — PASS; preserved evidence: 46 passed.
- `PROPTEST-STATE-001` — WAIVED; non-required secondary evidence not executed.

## Ledger Counts

- PASS: 12
- FAIL_LOCAL: 0
- WAIVED: 1
- Required open failures: 0

## Trusted Boundary

No broad `assume`, `external_body`, `external`, or `axiom` was introduced. Production-to-harness refinement remains explicit trusted boundary; storage/async/I/O/wall-clock/CLI are shell exclusions covered by non-Verus layers.

## Decision

State 12 APPROVED. Highest completed state: State 12. Next gate: landing/quality gates for the isolated workspace if code ownership proceeds; no Verus blocker remains for `vb-qi37.16.2`.
