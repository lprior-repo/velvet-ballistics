# Proof Plan Review Input: vb-qi37.4

STATUS: READY_FOR_PROOF_PLAN_REVIEW_AFTER_STATE_6_REPAIR

updated_at: 2026-05-15T17:53:20-05:00

## Files For Reviewer

- `.beads/vb-qi37.4/proof-strategy.md`
- `.beads/vb-qi37.4/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.4/contract.md`
- `.beads/vb-qi37.4/tla-spec.md`
- `.beads/vb-qi37.4/verification-layers.md`
- `.beads/vb-qi37.4/proof-obligations.jsonl`
- `.beads/vb-qi37.4/traceability-matrix.jsonl`
- `.beads/vb-qi37.4/proof-review.md`
- `.beads/vb-qi37.4/proof-findings.jsonl`
- `.beads/vb-qi37.4/proof-repair-guide.md`
- `.beads/vb-qi37.4/contract-verification-review.md`

## Review Questions

- Do rows 1-15 in `proof-obligations.planned.jsonl` coherently mirror the authoritative 15-row execution ledger in `proof-obligations.jsonl`?
- Is the revised State 5/6 proof policy acceptable: direct TLC/Verus commands are canonical for proof acceptance, while `moon run :verify-proof` is non-required tooling debt?
- Are later-owner obligations correctly classified as State 8 or State 11 rather than State 6 blockers?
- Are Flux, Miri-primary, Lean/Aeneas/Hax, proptest-primary, supply-chain, and Moon wrapper decisions sufficiently explicit and non-hallucinatory?

## Required Planned Proof Commands

- `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`
- `verus verification/verus/admission_artifact_model.rs`
- `verus verification/verus/capability_artifact_model.rs`

## Later Owner-State Commands

- `moon run :verify-deep`
- `moon run :lint-src`
- `moon ci`

## Non-Required Tooling Debt

- `moon run :verify-proof`: `CANONICAL-PROOF-GATE-016`, `required=false`, `status=waived`, `mode=waived`. Prior evidence shows Bash fails on `scripts/rust-verification-gauntlet.sh` shell-invalid `//!` lines before proof lanes run. Direct TLC/Verus rows are the accepted State 5/6 proof evidence policy; the wrapper remains tooling debt before final closure/release.

## Ledger Coherence Rule

- Authoritative execution ledger for State 6: `.beads/vb-qi37.4/proof-obligations.jsonl`.
- State 4 planning superset: `.beads/vb-qi37.4/proof-obligations.planned.jsonl`.
- Planned rows 1-15 must keep the same IDs and owner states as the execution ledger.
- Planned rows 16-21 are policy/waiver/not-applicable decisions and must not be cited as absent execution-ledger rows.

## Later-State Classification

- State 5 now-reviewable proof rows: `TLA-ACK-001`, `TLA-STATE-002`, `VERUS-CAP-003`, `VERUS-GATE-004`, `VERUS-DIGEST-005`.
- State 8 rows: `KANI-ADMIT-006`, `FUZZ-ARTIFACT-007`, `LOOM-JOURNAL-012`.
- State 11 rows: `INT-HEADER-008`, `INT-RECOVERY-009`, `STATIC-NO-YAML-010`, `MUT-ERR-011`, `GATE-CI-013`, `INT-DUPLICATE-014`, `INT-CAPACITY-015`.
- State 4 policy rows only: `CANONICAL-PROOF-GATE-016`, `FLUX-NOT-APPLICABLE-017`, `MIRI-WAIVE-018`, `LEAN-WAIVE-019`, `SUPPLY-NOT-APPLICABLE-020`, `PROPTEST-NOT-APPLICABLE-021`.

## Discovery Commands Run

- `pwd -P`: exit=0.
- `test -s ".beads/vb-qi37.4/contract.md" && test -s ".beads/vb-qi37.4/traceability-matrix.jsonl" && test -s ".beads/vb-qi37.4/delivery-scope.jsonl"`: exit=0.
- `jq -c . ".beads/vb-qi37.4/delivery-scope.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4/traceability-matrix.jsonl" >/dev/null && jq -c . ".beads/vb-qi37.4/proof-obligations.jsonl" >/dev/null`: exit=0.
- `jq -r '.expected_files[]' ".beads/vb-qi37.4/delivery-scope.jsonl" | while IFS= read -r path; do [ -e "$path" ] && printf '%s\n' "$path"; done | xargs -r rg -n "unsafe|unwrap\\(|expect\\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel"`: exit=0.
- `{ jq -r '.expected_files[]' ".beads/vb-qi37.4/delivery-scope.jsonl"; printf '%s\n' specs verification; } | while IFS= read -r path; do [ -e "$path" ] && printf '%s\n' "$path"; done | xargs -r rg -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe"`: exit=0.

## Planner Constraints Honored

- No production source edits.
- No test edits.
- No proof/model/harness/spec edits.
- No dependency/config edits.
- No source checkout writes.
- No Red Queen.
- No State 5 proof evidence or proof/model source edits.
