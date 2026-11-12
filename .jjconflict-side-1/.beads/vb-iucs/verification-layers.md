# Verification Layers: vb-iucs

| Layer | Target | Status |
|-------|--------|--------|
| Source inspection | Current main files contain proof integration source | PASS |
| Kani | Gate 8 accessor harnesses | PASS from recovered raw evidence |
| Kani | StepState runtime parity | PASS from recovered raw evidence |
| Verus | StepState transition mirror | PASS from recovered raw evidence |
| TLA+ TLC | BudgetArithmetic bounded arithmetic | PASS from recovered raw evidence |
| Moon CI | Whole workspace baseline after proof + CI repair | PASS from issue notes; rerun evidence recorded in machine gate report if available |
| Evidence audit | Prevent overclaiming deferred global obligations | APPROVED |

## Deferred

- Gate 8 Verus.
- Gate 8 Miri.
- Full validation pipeline composition `PO-030`.
