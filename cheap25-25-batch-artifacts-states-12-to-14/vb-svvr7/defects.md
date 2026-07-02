# Defects — vb-svvr7

## Bead: vb-svvr7 — IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)

### Phase: State 13 — Black Hat Review

### Status: APPROVED (no defects)

### Defect table

| ID | Severity | File:Line | Title | Status |
|----|----------|-----------|-------|--------|
| (none) | — | — | — | — |

### Notes

The black-hat review (`.beads/vb-svvr7/black-hat-review.md`) returned `STATUS: APPROVED` with no CRITICAL, HIGH, MEDIUM, or LOW findings. Three advisory notes were recorded as non-blocking:

- NOTE-1: `decode_postcard` is 34 lines (slightly above the 25-line Farley guideline) — linear flow, no ceremony benefit from splitting.
- NOTE-2: `encode_postcard` is 28 lines (slightly above the 25-line Farley guideline) — same rationale.
- NOTE-3: `PO-TB-PROP-01` is `BLOCKED_TOOLING` per `verification-ledger.jsonl:1`; compensating coverage is `PO-TB-UNIT-01` (21 passed). Documented as TB-TB-01 in `trusted-base-plan.md` §2.1; waiver recorded in `formal-waivers.jsonl:1`.

No defects. Implementation is approved for assurance bundling (State 14).