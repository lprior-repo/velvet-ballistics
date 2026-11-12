# Proof Repair Guide: vb-qi37.5.2

## Diagnosis
Proof artifacts for vb-qi37.5.2 are **missing entirely**. The proof-writer phase did not produce the required artifacts:
- `.beads/vb-qi37.5.2/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.5.2/proof-writer-report.md`
- `.beads/vb-qi37.5.2/proof-evidence.md`
- `.beads/vb-qi37.5.2/proof-strategy.md`

## Required Repairs

### 1. Execute proof-writer phase (BLOCKER)
**What:** Run the proof-writer skill to produce all required proof artifacts for "Enforce retry and replay idempotency".

**Artifacts to produce:**
- `proof-obligations.planned.jsonl` — planned proof obligations for retry/replay idempotency
- `proof-writer-report.md` — proof-writer summary with obligation-to-artifact mapping
- `proof-evidence.md` — raw verifier output, command evidence, pass/fail status per obligation
- `proof-strategy.md` — verification approach (Kani? Miri? proptest? TLA+?)

**Target file:** `.beads/vb-qi37.5.2/` (this directory)

### 2. Retry proof-reviewer after artifacts exist
Once the above 4 files exist in `.beads/vb-qi37.5.2/`, re-run proof-reviewer in phase 6.

## Routing
Route back to **proof-writer** to produce the missing artifacts before proof-reviewer can execute.

---
*proof-reviewer v1.0.1 | vb-qi37.5.2 | 2026-05-13*
