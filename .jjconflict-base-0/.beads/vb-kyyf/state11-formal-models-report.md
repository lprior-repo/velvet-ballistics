# vb-kyyf State 11 Formal Models Report

STATUS: APPROVED

## Scope
- Bead: `vb-kyyf`
- State: `11 formal-verifier`
- Sublane: `tla-verus-formal-models`
- Attempt: `4 of 7`
- Workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Manifest: `.beads/vb-kyyf/dispatch-state11-formal-models-attempt4.json`

## Startup Rules Cited
- `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: lines 21-24 require approved formal plan, every obligation accounted, scoped classification, and fail-closed missing required tools.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same content/version; per developer instruction this file wins if conflicts exist. No conflict observed.

## Frozen Inputs
- `.beads/vb-kyyf/proof-obligations.planned.jsonl`
- `.beads/vb-kyyf/proof-review.md`
- `.beads/vb-kyyf/contract-verification-review.md`
- `.beads/vb-kyyf/proof-evidence.md`

## Commands Executed
1. PO-008 / TLA-KYYF-001 / `VbKyyfReplayDeterminism`
   - Command: `JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp' tlc -workers 32 -metadir /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla`
   - Exit: `0`
   - Result: `PASS`
   - Evidence: TLC completed with no errors; `42907696 states generated, 16483704 distinct states found, 0 states left on queue`; depth `9`; finished in `06min 37s`.

2. PO-009 / VERUS-KYYF-001 / `verification/verus/vb_kyyf_normalization.rs`
   - Command: `verus verification/verus/vb_kyyf_normalization.rs`
   - Exit: `0`
   - Result: `PASS`
   - Evidence: `verification results:: 43 verified, 0 errors`.

## Artifact Outputs
- `.beads/vb-kyyf/state11-formal-models-report.md`
- `.beads/vb-kyyf/tla-report.md`
- `.beads/vb-kyyf/verus-report.md`
- `.beads/vb-kyyf/verification-ledger-formal-models.jsonl`

## Decision
Both required formal model obligations for this sublane passed on frozen inputs. No waivers used. No production, test, or proof files were modified.
