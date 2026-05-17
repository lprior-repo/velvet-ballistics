# Contract Verification Review

STATUS: APPROVED

## Independent Reviewer

- Bead: `vb-qi37.16.2`
- Workspace reviewed: `/home/lewis/src/Velvet-ballistics-vb-qi37-16-2-go`
- Source checkout `/home/lewis/src/Velvet-ballistics` was not used.
- Skill instructions read: `/home/lewis/.claude/skills/contract-verification-reviewer/SKILL.md` and `/home/lewis/.agents/skills/contract-verification-reviewer/SKILL.md`; both are version `1.5.0`, and the `.agents` copy is controlling if conflict exists.

## Files Reviewed

- `.beads/vb-qi37.16.2/contract.md`
- `.beads/vb-qi37.16.2/tla-spec.md`
- `.beads/vb-qi37.16.2/lean-contract.md`
- `.beads/vb-qi37.16.2/verification-layers.md`
- `.beads/vb-qi37.16.2/proof-obligations.jsonl`
- `.beads/vb-qi37.16.2/traceability-matrix.jsonl`
- `.beads/vb-qi37.16.2/verus-report.md`
- `.beads/vb-qi37.16.2/formal-verification-report.md`
- `.beads/vb-qi37.16.2/verification-ledger.jsonl`
- `.beads/vb-qi37.16.2/verus_resume_harness.rs`

## Command Evidence

- `test -s ... && jq -c . proof-obligations.jsonl && jq -c . traceability-matrix.jsonl && jq -c . verification-ledger.jsonl` -> exit 0; required artifacts present; JSONL valid.
- Python schema/trust validation over `proof-obligations.jsonl`, `verification-ledger.jsonl`, and `verus_resume_harness.rs` -> `schema_obs= 0`; required fields present, `owner_state`/`rerun_from` typed, Verus commands exact, TLA+ and Verus metadata present, Verus ledger PASS evidence present, no `assume(` / `external_body` / `external` / `axiom` trust tokens found.
- `verus .beads/vb-qi37.16.2/verus_resume_harness.rs` -> `verification results:: 6 verified, 0 errors`.

## Findings

- Severity: MINOR
  - Clause: `contract.md` line 79
  - Problem: `JurnalError` typo in contract signature.
  - Required fix: Correct to `JournalError` during ordinary artifact hygiene; this does not weaken proof obligations or State 12 approval.

No lethal or major defects found.

## Contract-Equivalence Decision

- The replacement of invalid standalone production-file Verus commands with `verus .beads/vb-qi37.16.2/verus_resume_harness.rs` is acceptable for the approved Verus scope: Rust-local pure/core resume predicates, pure append-only sequence behavior, hydration predicate, and typestate field presence.
- The harness is executable and independently re-ran successfully with `6 verified, 0 errors`.
- The repair does not pretend to prove production I/O, async scheduling, storage durability, wall-clock behavior, or CLI formatting. Those are explicitly excluded from Verus and covered by TLA+/replay/integration obligations.
- The production-to-harness refinement boundary is explicit in `verification-layers.md`, `verus-report.md`, `formal-verification-report.md`, and each Verus proof obligation `trusted_boundary` field.
- No fake proof mechanism was found in the harness: no `assume`, verifier external body, verifier external, or axiom token.

## Coverage Decision

- Contract clauses traced: YES; all PRE/POST/INV clauses appear in `traceability-matrix.jsonl` and proof obligations or second-ring evidence.
- TLA+-owned clauses covered: YES; TLA+ obligations include module/model/config, variables, actions, invariants, temporal properties, fairness, constraints, and refinement.
- Verus-owned clauses covered: YES; five Verus obligations target the executable bead-local harness with exact command, proof function names, invariants, shell exclusions, and explicit trusted boundaries.
- Theorem-owned clauses covered: YES; `lean-contract.md` explicitly assigns all Rust-local proof obligations to Verus and justifies no Lean/Aeneas/Hax projection.
- Proof obligations traced: YES; 13 obligations loaded, all required schema fields present.
- PASS evidence valid: YES; `verification-ledger.jsonl`, `verus-report.md`, and independent rerun agree on Verus PASS with `6 verified, 0 errors`.
- TLA+ scope valid: YES.
- Verus scope valid: YES.
- Lean/Aeneas/Hax scope valid: YES.
- Waivers valid: YES; only non-required `PROPTEST-STATE-001` is waived in the ledger, with required obligations PASS.

## State12 Approval

State12 approval may stand. The repaired Verus harness is contract-equivalent for the scoped pure obligations, executable, independently verified, and its trust boundary is explicit enough for downstream State12/landing decisions.
