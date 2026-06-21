# Questionable Findings Resolution Summary

The independent reviewer pass left 47 findings in `QUESTIONABLE` status. A
third adjudication pass re-read each questionable finding against production
source context and forced every item into one of two final states:

- `RESOLVED_CONFIRMED`: actionable after narrowed wording/severity where needed.
- `RESOLVED_REJECTED`: do not treat as a bug.

No findings remain in the `QUESTIONABLE` bucket.

## Resolution Totals

| Area | Questionable Entering Pass | Resolved Confirmed | Resolved Rejected |
|---|---:|---:|---:|
| `vb_storage` + cross-cutting | 10 | 1 | 9 |
| `vb_runtime` | 23 | 10 | 13 |
| `vb_core` | 14 | 6 | 8 |
| **Total** | **47** | **17** | **30** |

## Final Bug-Hunt Classification

| Area | Audited | Final Confirmed | Final Rejected | Remaining Questionable |
|---|---:|---:|---:|---:|
| `vb_storage` + cross-cutting | 58 | 43 | 15 | 0 |
| `vb_runtime` | 113 | 73 | 40 | 0 |
| `vb_core` | 60 | 39 | 21 | 0 |
| **Total** | **231** | **155** | **76** | **0** |

## Detailed Reports

- `resolution-pass/storage-questionables.md`
- `resolution-pass/runtime-questionables.md`
- `resolution-pass/core-questionables.md`

## Use This Going Forward

Treat the 155 final confirmed findings as the actionable backlog. Treat the 76
final rejected findings as non-work unless a future code or contract change
creates new evidence.
