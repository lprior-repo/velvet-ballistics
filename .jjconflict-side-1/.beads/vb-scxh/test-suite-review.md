# Test Suite Review: vb-scxh State 8 Audit Harness / Manifest

STATUS: APPROVED

## Basis

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 113-180 define Mode 2 suite/static review concerns; lines 265-278 define severity and full rerun after rejection; lines 329-338 require evidence-only approval and exact findings.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content; `.agents` wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 13-20 require traceable behavior evidence; lines 32-48 reject weak generated/marker coverage; lines 178-191 require failure locality.

## Verdict

APPROVED for State 9. The repaired State 8 harness is adequate scaffolding to proceed to State 11 raw evidence/audit execution. It no longer launders weak Moon CI or mutation evidence into prelim-pass, and it still does not claim State 11/12/final evidence success.

## Findings

- LETHAL: 0
- MAJOR: 0
- MINOR: 0

## Repaired Rejection Items

1. Moon CI marker gate repaired: `state8-audit-harness.py:145-184` requires command/status/task/test/runtime markers plus artifact-path evidence marker and fresh-rerun marker before `PASS_PRELIM`; current generated preflight stays red with `missing_markers=['artifact path evidence marker', 'fresh rerun marker']` at `state8-red-preflight.md:80-91`.
2. Mutation marker gate repaired: `state8-audit-harness.py:187-209` requires exact `35/35 unviable` markers in both mutation report and verification ledger, while keeping `FAIL_UNVIABLE` / `DEFERRED` as non-adequacy; current generated preflight stays red with exact missing-marker evidence at `state8-red-preflight.md:93-105`.

## Passing Harness Checks

- State 8 output remains explicitly preliminary, not final evidence: `state8-audit-harness.py:277-308`, `state8-red-preflight.md:1-7`.
- Workspace and approved-input checks have exact prelim evidence: `state8-red-preflight.md:11-37`.
- Required State 1/2 and referenced `vb-gvmt` artifacts are checked for non-empty presence: `state8-audit-harness.py:106-127`, `state8-red-preflight.md:39-50`.
- False-closure BD audit is a State 11 raw command plan, not inferred in State 8: `state8-audit-manifest.jsonl:3`, `state8-audit-harness.py:242-248`, `state8-red-preflight.md:52-63`.
- Safety anchor raw preflight preserves the downstream `BLOCK_LOCAL` failure instead of approving closure: `state8-audit-harness.py:130-142`, `state8-red-preflight.md:65-78`.
- Scope-control BD capture remains deferred to State 11 raw evidence: `state8-audit-manifest.jsonl:7`, `state8-audit-harness.py:252-258`, `state8-red-preflight.md:107-118`.
- Subagent-laundering and premature close/unblock are negative fixtures, not final State 12 decisions: `state8-audit-manifest.jsonl:8`, `state8-audit-manifest.jsonl:10`, `state8-red-preflight.md:120-158`.
- TLA canonical path preflight checks `.beads/vb-scxh/tla/ScxhRecovery.*` and rejects active `.beads/vb-scxh/specs/` obligation targets: `state8-audit-harness.py:212-230`, `state8-red-preflight.md:133-145`.
- Harness refuses final artifact names: `state8-audit-harness.py:324-327`.

## Downstream Blockers Preserved for State 11/12

- Exact 12 false-closure IDs must be derived from raw BD output, not inferred from prose.
- Safety bundle/ref remains a real State 11/12 `BLOCK_LOCAL` unless raw verification is repaired or formally waived.
- Moon CI needs raw artifact-path evidence and fresh rerun marker before the lane can be accepted.
- Mutation lane needs exact `35/35 unviable` raw markers while remaining `FAIL_UNVIABLE` / `DEFERRED`, not mutation adequacy PASS.
- Scope-control ownership for generated parity must be captured from raw `vb-gvmt` / `vb-qi37.10` BD output.
- State 12 must still produce Truth Serum/final decision and reject subagent-only evidence.

## Routing

- owner_state: State 11
- rerun_from: State 11 raw evidence/audit execution

## Artifact Paths

- Reviewed: `.beads/vb-scxh/state8-audit-manifest.jsonl`
- Reviewed: `.beads/vb-scxh/state8-audit-harness.py`
- Reviewed: `.beads/vb-scxh/state8-red-preflight.md`
- Reviewed: `.beads/vb-scxh/test-writer-report.md`
- Wrote: `.beads/vb-scxh/test-suite-review.md`
- Wrote: `.beads/vb-scxh/test-repair-guide.md`
