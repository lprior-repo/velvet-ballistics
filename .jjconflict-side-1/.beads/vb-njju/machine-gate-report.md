# Machine Gate Report — vb-njju

STATUS: PASS

## Bead
- id: vb-njju
- phase: 11 (formal-verification)
- isolated workspace: /home/lewis/src/femdation-vb-njju

## Gate Result
All 12 proof obligations executed and recorded.

| Obligation | Layer | Result |
|---|---|---|
| BDD-CAT-001 | proptest | PASS |
| MUT-ADM-001 | cargo-mutants | PASS |
| MUT-PLAN-002 | cargo-mutants | PASS |
| FUZZ-SMOKE-001 | cargo-fuzz | PASS |
| FUZZ-BUILD-002 | cargo-fuzz | PASS |
| PROP-TAINT-001 | proptest | PASS |
| PROP-REPLAY-002 | proptest | PASS |
| BOUNDARY-FUZZ-001 | cargo-fuzz | PASS |
| BOUNDARY-REL-002 | gauntlet-all | PASS |
| TRACE-JSONL-001 | static-scan | PASS |
| TLA-WAIVE-001 | waiver | WAIVED |
| LEAN-WAIVE-001 | waiver | WAIVED |

## Formal Verification Report
- path: .beads/vb-njju/formal-verification-report.md
- STATUS: APPROVED

## Verification Ledger
- path: .beads/vb-njju/verification-ledger.jsonl
- 12 obligation records

## Conclusion
vb-njju formal verification gate: PASS
