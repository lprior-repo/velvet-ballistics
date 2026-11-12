bead_id: vb-scxh
bead_title: Recover false 12-bead closure and restore green CI
phase: 2
updated_at: 2026-05-14T00:00:00Z
attempt: 1-of-7

# State 2 Codebase Map — Best-Effort Note

`explore` completed read-only discovery from `/home/lewis/src/vb-scxh` but could not write files because its active role instructions forbade filesystem modification. Go-skill records this as the State 2 best-effort codebase-map note permitted by the checklist when the normal codebase-map cannot be produced. No production, test, proof, or contract files were modified.

## Scope

- Bead status observed by `bd --db /home/lewis/src/.beads/dolt show vb-scxh --json`: `in_progress`.
- Purpose: Truth Serum recovery of false 12-bead closure and restoration of green CI.
- Blocks: `vb-engine-yaml`.
- Nature: evidence-integrity/recovery bead, not new UI/codegen implementation for the user's no-UI/no-codegen engine goal.

## Existing State 1 Artifacts

- `.beads/vb-scxh/STATE.md`
- `.beads/vb-scxh/baseline-report.md`

## Recovery Evidence Mentioned In Bead Notes

- False 12-bead closures were reopened/linked.
- Safety bundle preserved: `/home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle`.
- Safety bookmark: `rescue-vb-scxh-ci-green-20260513T030158Z`.
- Green `moon ci` evidence after `vb_codegen` repair.
- Remaining generated-mode parity gaps tracked by `vb-gvmt` / `vb-qi37.10`.

## Relevant Existing Evidence/Artifact Paths

- `.beads/vb-gvmt/parity-test-report.md`
- `.beads/vb-gvmt/moon-ci-or-static-scan-report.md`
- `.beads/vb-gvmt/formal-verification-report.md`
- `.beads/vb-gvmt/verification-ledger.jsonl`
- `.beads/vb-gvmt/contract.md`
- `.beads/vb-gvmt/proof-obligations.jsonl`
- `.beads/vb-gvmt/tla-report.md`
- `.beads/vb-gvmt/kani-report.md`
- `.beads/vb-gvmt/mutation-report.md`

## Source/Test/Proof Files Implicated By Recovery

- `crates/vb_codegen/src/lib.rs`
- `crates/vb_codegen/src/tests.rs`
- `crates/vb_codegen/src/kani_generated_runtime.rs`
- `crates/vb_codegen/tests/compile-fail/pass/minimal_workflow.rs`
- `crates/vb_expr/src/eval.rs`
- `.beads/vb-gvmt/specs/GeneratedParity.tla`
- `.beads/vb-gvmt/specs/GeneratedParity.cfg`
- `.beads/vb-gvmt/proofs/generated_semantics_verus.rs`

## Raw Evidence Claims To Audit Downstream

- `moon ci`: PASS, 19 tasks, 8276 tests passed, recorded in `.beads/vb-gvmt/moon-ci-or-static-scan-report.md`.
- POST parity: PASS, `post_011` 4 tests passed; broader `post_` 33 tests passed.
- TLA+: PASS, 17 states generated, 13 distinct states.
- Verus: PASS, 6 verified, 0 errors.
- Kani: PASS, 5 harnesses verified.
- Mutation: `FAIL_UNVIABLE` / deferred; not mutation adequacy.

## Risks And Open Questions

- Mutation adequacy is not satisfied; cargo-mutants produced only unviable mutants.
- Generated parity is representative/limited, not exhaustive over all supported workflow shapes.
- Remaining generated-mode gaps are intentionally tracked by open beads, especially `vb-gvmt` / `vb-qi37.10`.
- `vb-engine-yaml` remains blocked until recovery evidence is canonically packaged/validated by downstream Go-skill states.
- State 3 must distinguish evidence-recovery requirements from new implementation requirements, and downstream evidence packaging must distinguish raw command evidence from subagent claims.

## State 2 Completeness

Best-effort codebase map recorded. Canonical `delivery-scope.jsonl` is mandatory before State 3.
