# vb-kyyf State 11 CAP Formal Models Report

STATUS: APPROVED

## Scope
- Bead: `vb-kyyf`
- State: `11`
- Sublane: `cap-unblock-tla-verus-formal-models`
- Workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Manifest: `.beads/vb-kyyf/dispatch-state11-cap-formal-models.json`
- Executed obligations: `PO-008`, `PO-009`

## Startup Rules Cited
- `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: lines 14, 21-24, 30-31, 56, 100-109 require executing existing approved obligations, approved review gate, accounting with PASS/FAIL/WAIVED/DEFERRED, exact named second-ring commands, no hallucinated evidence, and fail-closed TLA+/Verus handling.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same content/version; per instruction this file wins on conflict. No conflict observed.

## Input Gate
- `.beads/vb-kyyf/proof-obligations.planned.jsonl`: present and JSONL-valid.
- `.beads/vb-kyyf/proof-review.md`: `STATUS: APPROVED`.
- `.beads/vb-kyyf/contract-verification-review.md`: `STATUS: APPROVED`.
- `verification/tla/VbKyyfReplayDeterminism.tla`: present.
- `verification/tla/VbKyyfReplayDeterminism.cfg`: present.
- `verification/verus/vb_kyyf_normalization.rs`: present.

## Results
| Obligation | Layer | Command | Result | Evidence |
|---|---|---|---|---|
| `PO-008` | TLA+ / TLC | `JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp' tlc -workers 32 -metadir /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla` | `PASS` | TLC completed with no error; `42,907,696` states generated, `16,483,704` distinct, depth `9`, 0 states left on queue. |
| `PO-009` | Verus | `verus verification/verus/vb_kyyf_normalization.rs` | `PASS` | `verification results:: 43 verified, 0 errors`. |

## Artifact Outputs
- `.beads/vb-kyyf/state11-cap-formal-models-report.md`
- `.beads/vb-kyyf/tla-report.md`
- `.beads/vb-kyyf/verus-report.md`
- `.beads/vb-kyyf/verification-ledger-cap-formal-models.jsonl`

## Residual Risk
- None for this sublane. The planned Verus expected count was `42 verified`; current exact command verified `43` obligations with `0 errors`, which remains stronger/equivalent pass evidence.
