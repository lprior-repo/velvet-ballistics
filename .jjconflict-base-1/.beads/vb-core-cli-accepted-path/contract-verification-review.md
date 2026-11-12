# Contract Verification Review

STATUS: REJECTED

## Files Reviewed

- Startup authority: `/home/lewis/.claude/skills/contract-verification-reviewer/SKILL.md` and `/home/lewis/.agents/skills/contract-verification-reviewer/SKILL.md`; both read, no conflict observed, `.agents` would win.
- `.beads/vb-core-cli-accepted-path/contract.md`
- `.beads/vb-core-cli-accepted-path/tla-spec.md`
- `.beads/vb-core-cli-accepted-path/lean-contract.md`
- `.beads/vb-core-cli-accepted-path/verification-layers.md`
- `.beads/vb-core-cli-accepted-path/proof-obligations.jsonl`
- `.beads/vb-core-cli-accepted-path/traceability-matrix.jsonl`
- `.beads/vb-core-cli-accepted-path/proof-obligations.planned.jsonl`
- `.beads/vb-core-cli-accepted-path/proof-writer-report.md`
- `.beads/vb-core-cli-accepted-path/proof-evidence.md`
- `.beads/vb-core-cli-accepted-path/proof-review.md`
- `.beads/vb-core-cli-accepted-path/proof-findings.jsonl`

## Command Evidence

- `test -s` gate for contract.md, tla-spec.md, lean-contract.md, verification-layers.md, proof-obligations.jsonl, traceability-matrix.jsonl, proof-obligations.planned.jsonl, proof-writer-report.md, and proof-evidence.md -> exit 0.
- `jq -c .` for proof-obligations.jsonl, traceability-matrix.jsonl, and proof-obligations.planned.jsonl -> exit 0.
- `test -s proof-review.md proof-findings.jsonl && jq -c . proof-findings.jsonl` -> exit 0.
- Schema/status scan over `proof-obligations.jsonl` -> no missing required fields, all rows status `planned`, TLA+ row has required TLA fields.
- Required-status scan over `proof-obligations.planned.jsonl` -> output `PO-007:blocked_tooling`.

## Findings

- Severity: LETHAL
  - Clause: `KANI-ADMISSION-001` / `PO-007` covering `ERR-001,ERR-002,ERR-003,ERR-004,ERR-007,POST-004`.
  - Problem: Required Kani/aggregate malformed decode/admission/bypass obligation remains `blocked_tooling` after State 5. `proof-evidence.md` and `proof-review.md` show `moon run :verify-proof` exits 2 before Kani executes due `scripts/rust-verification-gauntlet.sh` shell parse errors on leading `//!` lines. No approved waiver exists.
  - Required fix: Repair the verify-proof gauntlet so Kani executes and record raw PASS evidence, or add an explicit reviewer-approved waiver with owner, reason, expiry, limitation, and compensating evidence.
- Severity: MAJOR
  - Clause: `VERUS-ADMISSION-001` / `PO-004`.
  - Problem: Proof review records a traceability mismatch between required proof names in `proof-obligations.jsonl` and actual Verus proof function names, and warns the artifact is still a verifier-only classifier detached from executable admission code.
  - Required fix: Align obligation proof names with the Verus artifact or repair the artifact names, then require downstream implementation/formal evidence binding the verifier-only model to real admission code before runtime correctness is claimed.

## Coverage Decision

- Contract clauses traced: yes, via `traceability-matrix.jsonl`.
- TLA+-owned clauses covered: planned and locally evidenced by TLC, but downstream approval blocked by unresolved required PO-007.
- Verus-owned clauses covered: planned and locally evidenced for PO-002..PO-004, with PO-004 traceability caveat.
- Theorem-owned clauses covered: no theorem kernel required; waiver shape is acceptable for current scope.
- Proof obligations traced: not fully acceptable because required `PO-007` is blocked, unexecuted, and unwaived.
- TLA+ scope valid: yes for contract adequacy.
- Verus scope valid: partial; PO-004 traceability/model-realization caveat must be repaired or explicitly carried.
- Lean/Aeneas/Hax scope valid: yes; no shell theorem claim present.
- Waivers valid: theorem/Miri waivers acceptable; no valid waiver exists for required `PO-007`.

## Decision

State 6 cannot approve. Required proof-obligation evidence is incomplete: `PO-007` is mandatory, high-risk, unexecuted, and unwaived. Route back to State 5/tooling repair or produce an explicit approved waiver before retrying State 6.
