# Contract Verification Review

STATUS: REJECTED

## Startup Skill Authority
- Read `/home/lewis/.claude/skills/contract-verification-reviewer/SKILL.md`: version 1.5.0; requires real `test -s`/`jq` evidence, TLA+ for temporal recovery behavior, Verus-first Rust-local coverage, executable obligations, and binary approval.
- Read `/home/lewis/.agents/skills/contract-verification-reviewer/SKILL.md`: same version/content; this copy is authoritative if conflicts exist.

## Files Reviewed
- `.beads/vb-qi37.1.6/contract.md`
- `.beads/vb-qi37.1.6/tla-spec.md`
- `.beads/vb-qi37.1.6/lean-contract.md`
- `.beads/vb-qi37.1.6/verification-layers.md`
- `.beads/vb-qi37.1.6/proof-obligations.jsonl`
- `.beads/vb-qi37.1.6/traceability-matrix.jsonl`
- `.beads/vb-qi37.1.6/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.1.6/proof-writer-report.md`
- `.beads/vb-qi37.1.6/proof-evidence.md`
- `.beads/vb-qi37.1.6/proof-review.md`

## Command Evidence
- `test -s ... && jq -c . ...` over all requested review artifacts and required JSONL files -> exit 0; mandatory files exist and JSONL parses.
- `jq` schema/status check on `proof-obligations.jsonl` -> exit 0; no missing required schema fields, no non-`planned` statuses, and `TLA-REC-001` has required TLA+ shape fields.
- `jq -r 'select(.required == true and (.status != "planned")) | .id + ":" + .status' .beads/vb-qi37.1.6/proof-obligations.planned.jsonl` -> `PO-009:blocked_tooling`, `PO-015:blocked_tooling`.

## Findings

- Severity: LETHAL
  - Clause: `TLA-REC-001` / `PO-001` / `PO-015`; contract clauses `PRE-002`, `PRE-003`, `POST-002` through `POST-007`, `INV-002`, `INV-003`, `INV-004`, `INV-007`.
  - Problem: The bead is temporal by contract, but executable TLA+ evidence is still blocked. `proof-evidence.md` records `java -jar tla2tools.jar verification/tla/RecoveryCrashRestart.tla -config verification/tla/RecoveryCrashRestart.cfg` exit 1 with `Error: Unable to access jarfile tla2tools.jar`; `proof-review.md` independently rejects the same. `PO-015` is `required: true`, `status: blocked_tooling`, and its waiver expires before State 6 approval.
  - Required fix: Provide TLC tooling or an equivalent checked TLA+ runner in the isolated workspace and record raw PASS evidence for invariants, deadlock stance, and liveness; then update planned obligation status/evidence.

- Severity: LETHAL
  - Clause: `GATE-REC-001` / `PO-009`; `ALL`.
  - Problem: Canonical proof gate remains non-executable. `proof-evidence.md` records `moon run :verify-proof` exit 2 before reaching scoped artifacts because `scripts/rust-verification-gauntlet.sh` is parsed as shell and errors on Rust doc-comment lines. `PO-009` is `required: true`, `status: blocked_tooling`, and its waiver expires before State 6 approval.
  - Required fix: Repair or correctly invoke the canonical proof gate so it reaches TLA+/Verus lanes and records scoped PASS/approved-waiver evidence.

- Severity: MAJOR
  - Clause: `KANI-REC-001` / `PO-003`; `PRE-004`, `PRE-006`, `POST-008`, `INV-001`, `INV-005`, `INV-006`.
  - Problem: The refreshed proof plan marks Kani bounded state/error-classification evidence as `required: true`, `status: planned`, `waiver: null`, but State 5 evidence provides no harness artifact, execution output, or explicit deferral/waiver. This is not an approval blocker by itself for the base `proof-obligations.jsonl` schema, but it remains a required planned lane with no evidence for State 6 release.
  - Required fix: Execute the Kani lane, replace it with Verus-discharge evidence and an explicit waiver/deferral record, or mark it as later-state work with owner, expiry, reason, and compensating evidence.

## Coverage Decision
- Contract clauses traced: APPROVED at repaired contract-artifact level; `PRE-006` now appears in `VERUS-REC-001`, `INT-REC-002`, `MUT-REC-001`, and traceability.
- TLA+-owned clauses covered: REJECTED for State 6 approval; planned shape is present, but required executable TLC evidence is blocked.
- Verus-owned clauses covered: APPROVED as local supporting evidence; direct Verus proof reports `10 verified, 0 errors`, with production mapping artifact reviewed.
- Theorem-owned clauses covered: APPROVED; Lean/Aeneas/Hax waiver has owner, reason, expiry, and compensating evidence.
- Proof obligations traced: REJECTED; refreshed planned obligations contain required `blocked_tooling` rows that expire before approval.
- TLA+ scope valid: REJECTED until TLC or equivalent model-check output exists.
- Verus scope valid: APPROVED for contract-review purposes; still not a substitute for TLA/canonical gate evidence.
- Lean/Aeneas/Hax scope valid: APPROVED.
- Waivers valid: REJECTED for `PO-009` and `PO-015` because their own expiry forbids State 6 approval while tooling remains blocked.
