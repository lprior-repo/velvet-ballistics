# Truth Serum Report: vb-qi37.8

**bead_id**: vb-qi37.8
**audited**: 2026-05-17
**scope**: current-tree proof repair evidence.

## Status

**APPROVED**

Truth-serum re-audited the corrected artifacts and returned `APPROVE` after durable Verus/TLC raw evidence files were added and traceability was narrowed to scoped claims.

## Evidence Audit

| Claim | Durable Evidence | Verdict |
|-------|------------------|---------|
| Gate 8 bounded valid accessors pass | `/home/lewis/.local/share/opencode/tool-output/tool_e34ef1482001qcOlXtLV6oho6J` | PASS |
| Gate 8 zero accessors pass | `/home/lewis/.local/share/opencode/tool-output/tool_e34f581fb001fuSDAdY6gUn2ug` | PASS |
| Gate 8 index-only accessors without symbols pass | `/home/lewis/.local/share/opencode/tool-output/tool_e34f5b38d0013LAew53aQImHU4` | PASS |
| Gate 8 bounded inputs do not panic | `/home/lewis/.local/share/opencode/tool-output/tool_e34f700520011ShVGRxe0Y3Jzl` | PASS |
| Gate 8 field symbol out-of-bounds rejects | `/home/lewis/.local/share/opencode/tool-output/tool_e34fa8963001IInVFbayevs0LH` | PASS |
| Gate 8 `u32::MAX` index rejects | `/home/lewis/.local/share/opencode/tool-output/tool_e34fab23a001J3G2O3ssQdacfZ` | PASS |
| Gate 8 root out-of-bounds rejects | `/home/lewis/.local/share/opencode/tool-output/tool_e34faec38001KSsoRuQHUd5m1B` | PASS |
| StepState Kani parity | `/home/lewis/.local/share/opencode/tool-output/tool_e34fbcc37001x2PAzA97hgWznY` | PASS |
| StepState Verus mirror | `.beads/vb-qi37.8/evidence/verus-step-state-machine.out` | PASS |
| BudgetArithmetic TLC model | `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out` | PASS |

## Laundering Checks

| Risk | Verdict |
|------|---------|
| Stale raw paths reused | PASS |
| Missing harness names claimed | PASS |
| `PO-030` overclaimed as covered | PASS |
| Gate 8 Verus claimed without execution | PASS |
| Broad historical coverage laundered into current repair | PASS |

## Verdict

**TRUTH-SERUM VERDICT: APPROVED**

The current evidence bundle is auditable for the scoped repair. `PO-030` remains deferred and is not covered by Gate 8 evidence.
