# Metadata Correction — vb-hp1qq

## Purpose

Correct false zero-test claim in vb-hrvix contract and disposition independent review findings F1/F2 (code) and T1 (test).

## What was wrong

`.beads/vb-hrvix/contract.md` cited the command:
```bash
cargo test -p velvet-ballistics --lib -- workflow_input_slots --exact --test-threads=1
```
with exit 0 as evidence that the 6 constructor input-slot tests passed.

This command ran **0 tests** because:
1. Wrong package: tests live in `crates/vb_core`, not `velvet-ballistics`
2. Wrong filter: `--exact` with `workflow_input_slots` matches no test (real names are `workflow_input_slots_accept_*` etc.)

A zero-test filtered run with exit 0 is not evidence (test-reviewer gate 8).

## What was corrected

### 1. Contract annotation

`.beads/vb-hrvix/contract.md` now has:
- A corrected test row showing the actual command and result (7 passed)
- A "Metadata Correction" section explaining the error

### 2. Fresh evidence

New immutable evidence at `.beads/vb-hp1qq/evidence/`:
- `actual-input-slot-tests.*` — `cargo test -p vb_core --lib input_slot -- --test-threads=1` → 7 passed, 0 failed (6 new constructor cases + 1 pre-existing engine case)
- `production-inner-drift.*` — drift gate PASS (64 mirrors, 78 extern files)
- `verus-production-binding.*` — binding gate PASS (STRONG 0, WEAK 72, VACUUM 0)

All run through `/tmp/vb-run-gate.py` with `CARGO_BUILD_JOBS=2`.

### 3. F1/F2/T2 disposition (via vb-9tg1c author fixes)

- **F1:** Drift annotation added above `validate_input_slots` in production_inner; stale "all stub Ok" comments corrected.
- **F2:** Variant count comment updated 8 → 11.
- **T2:** Left as-is (optional field-level assertions; not required for landing).

## What was NOT changed

- Old raw evidence files in `.beads/vb-hrvix/evidence/` remain unchanged (only documentation corrected)
- No production source files modified
- No test strengthening
- No full CI or fleet Kani runs

## Trust boundary

The corrected evidence proves the 7 tests pass. The Verus proofs remain spec-level (not whole-production verification). The production_inner mirror binding is WEAK (identifier-level drift heuristic), unchanged by these corrections.