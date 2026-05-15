# State 11 Repair Routing: vb-scxh

STATUS: BLOCKED

## Startup / Authority

- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md`; both are formal-verifier v1.5.0 and require real command evidence, fail-closed missing required evidence, and no invented PASS. No conflict observed; `/home/lewis/.agents/...` would win if there were one.
- Workspace validation: `pwd -P` returned `/home/lewis/src/vb-scxh`.
- JSONL validation: `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `delivery-scope.jsonl`, `verification-ledger.jsonl`, and referenced `vb-gvmt/verification-ledger.jsonl` parsed with `jq -c .`.
- Artifact presence validation passed for required State 11 inputs and referenced `vb-gvmt` reports.

## Exact Blockers

1. `SAFETY-SCXH-001` / `ERR-SCXH-006`: `FAIL_LOCAL` / `BLOCK_LOCAL`.
   - Final low-output probe on 2026-05-14: `test -e /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle` returned nonzero; `git bundle verify /home/lewis/src/Velvet-ballistics-rescue-20260513T022011Z.bundle` failed with `error: could not open ...`.
   - `git show-ref rescue-vb-scxh-ci-green-20260513T030158Z` exited `1` in both `/home/lewis/src/vb-scxh` and read-only `/home/lewis/src/Velvet-ballistics`.
   - `glob **/*.bundle` under `/home/lewis/src` and `/tmp/opencode` found no bundle files.
   - Plausible alternate refs found but not acceptable as the required exact anchor: `origin/rescue-main-pre-recovery-20260513T022011Z e1d254daf`, `origin/rescue/vb-zdxm-base-20260511T161708Z dd9ceba60`, `main/origin/main c6272854a`, `origin/release-clean-main 3b3d4218d`.
   - Required owner action: restore exact bundle/ref, or explicitly approve a waiver accepting missing-anchor risk and naming alternate immutable evidence. `formal-waivers.candidate.jsonl` remains `CANDIDATE_ONLY_NOT_APPROVED` and cannot unblock State 12.
2. State 12 rows remain intentionally blocked and were not executed: `TRUTH-SCXH-001`, `ERR-SCXH-003`, `ERR-SCXH-004`, `ERR-SCXH-009`.

## Fresh Passing Evidence Not Sufficient For Safety Anchor

- `CI-SCXH-001` is now PASS per fresh Moon CI evidence: 21 actions completed; 8185 tests passed; 6 skipped.
- This removes the prior CI blocker but does not satisfy `SAFETY-SCXH-001` / `ERR-SCXH-006`, because safety-anchor obligations require the exact bundle and exact ref or an owner-approved waiver.

## Parallel Next Wave

These can run without overlap; none should proceed to State 12.

- Safety-anchor restoration agent: restore only the exact bundle and rescue ref, or obtain owner-signed alternate immutable anchor waiver evidence. Writes only `safety-anchor-report.md` and candidate waiver notes if restoration fails.
- Ledger/audit agent: validate all `.beads/vb-scxh/*.jsonl`, reconcile `verification-ledger.jsonl` counts, and keep safety rows as `FAIL_LOCAL` unless raw repair passes or an approved waiver exists.

## Do Not Do

- Do not mark safety anchor waived from `formal-waivers.candidate.jsonl`.
- Do not treat fresh Moon CI PASS as substitute evidence for missing exact safety anchor.
- Do not proceed to State 12.
