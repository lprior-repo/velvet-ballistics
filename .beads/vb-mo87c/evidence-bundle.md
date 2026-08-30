# Evidence Bundle: vb-mo87c — Landing push and bead sync log audit

## Scope
Audit all landing reports for raw `git push`/`git status`/`bd dolt push`/`bd close` stdout/stderr/exit-code evidence. Attach raw evidence where available; downgrade claims that lack it.

## Audit method
Scanned all `landing-report.md` files under `.beads/` for raw command evidence markers:
- `exit_code` / `exit code` / `exit_status`
- `raw.stdout` / `raw.stderr` / `--- stdout` / `--- stderr`
- Full terminal output blocks with tool version lines

Scanned combined landing evidence files:
- `.beads/qi37-all-landing-evidence.md`
- `.beads/qi37-combined-landing-evidence.md`

## Results

### Landing reports audited: 48

| Bead ID | Has Raw Push/Sync Evidence | Classification |
|---------|--------------------------|----------------|
| vb-2b4g | NO | DOWNGRADED - UNSUPPORTED |
| vb-5m8w | NO | DOWNGRADED - UNSUPPORTED |
| vb-8cw4 | NO | DOWNGRADED - UNSUPPORTED |
| vb-c1s0 | NO | DOWNGRADED - UNSUPPORTED |
| vb-c3k9 | NO | DOWNGRADED - UNSUPPORTED |
| vb-cd6t | NO | DOWNGRADED - UNSUPPORTED |
| vb-core-atomic-admission | NO | DOWNGRADED - UNSUPPORTED |
| vb-core-lower-control-primitives | NO | DOWNGRADED - UNSUPPORTED |
| vb-core-lower-coverage-matrix | NO | DOWNGRADED - UNSUPPORTED |
| vb-core-lower-values-actions-refs | NO | DOWNGRADED - UNSUPPORTED |
| vb-core-storage-artifact-store | NO | DOWNGRADED - UNSUPPORTED |
| vb-core-trigger-contract | NO | DOWNGRADED - UNSUPPORTED |
| vb-e4mt | NO | DOWNGRADED - UNSUPPORTED |
| vb-engine-yaml | NO | DOWNGRADED - UNSUPPORTED |
| vb-f04l | NO | DOWNGRADED - UNSUPPORTED |
| vb-f7k6 | NO | DOWNGRADED - UNSUPPORTED |
| vb-hs9m | NO | DOWNGRADED - UNSUPPORTED |
| vb-hxm0 | NO | DOWNGRADED - UNSUPPORTED |
| vb-ib8i | NO | DOWNGRADED - UNSUPPORTED |
| vb-jpq7-proof-wave1 | NO | DOWNGRADED - UNSUPPORTED |
| vb-kyyf | NO | DOWNGRADED - UNSUPPORTED |
| vb-m5gp | NO | DOWNGRADED - UNSUPPORTED |
| vb-ogwh | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.13 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.14.1 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.1.4 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.15.3 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.22 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.23 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.2.4 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.2.5 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.25 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.4.2 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.5.4 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.6 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.8 | NO | DOWNGRADED - UNSUPPORTED |
| vb-qi37.9.2 | NO | DOWNGRADED - UNSUPPORTED |
| vb-ssei | NO | DOWNGRADED - UNSUPPORTED |
| vb-te1i | NO | DOWNGRADED - UNSUPPORTED |
| vb-tw3b | NO | DOWNGRADED - UNSUPPORTED |
| vb-v7x6 | NO | DOWNGRADED - UNSUPPORTED |
| vb-vcmq | NO | DOWNGRADED - UNSUPPORTED |
| vb-vt2f | NO | DOWNGRADED - UNSUPPORTED |
| vb-xi2f.10 | NO | DOWNGRADED - UNSUPPORTED |
| vb-ybi5 | NO | DOWNGRADED - UNSUPPORTED |
| vb-zioy | NO | DOWNGRADED - UNSUPPORTED |
| vb-zrop | NO | DOWNGRADED - UNSUPPORTED |

### Combined landing evidence files audited: 2

| File | Has Raw Push/Sync Evidence | Classification |
|------|--------------------------|----------------|
| .beads/qi37-all-landing-evidence.md | NO | DOWNGRADED - UNSUPPORTED |
| .beads/qi37-combined-landing-evidence.md | NO | DOWNGRADED - UNSUPPORTED |

## Summary
- **Total landing reports audited:** 48
- **With raw push/sync evidence:** 0
- **Downgraded (lacked raw evidence):** 48
- **Combined evidence files audited:** 2
- **Combined files downgraded:** 2
- **Total claims downgraded:** 50

## Downgrade methodology
Each landing report was checked for:
1. Raw `git push` stdout/stderr (branch names, remote URLs, commit hashes from terminal output)
2. Raw `git status` output (e.g., `## main...origin/main — nothing to commit`)
3. Raw `bd dolt push` output (e.g., `Push complete`, `remote: ...`)
4. Raw `bd close` output (full command with exit status)
5. Explicit `exit_code` or `exit_status` fields tied to push/close commands

None of the 48 landing reports contained any of the above. The exit codes referenced in vb-qi37.14.1 (lines 44-55) are CLI exit code contract definitions, not push command exit statuses.

## Sample findings

### vb-zrop (line 11-12 of landing-report.md)
```
- Beads sync: `bd dolt push` succeeded.
- Remote push: `git push` succeeded.
```
No raw `bd dolt push` or `git push` output. The word "succeeded" is an assertion, not evidence. **Downgraded to UNSUPPORTED.**

### vb-hxm0 (line 10, 12 of landing-report.md)
```
- git push origin HEAD:main: succeeded.
- bd dolt push: Push complete.
```
No raw stdout/stderr. "Push complete" is a claim, not a logged terminal line. **Downgraded to UNSUPPORTED.**

### vb-ib8i (lines 11-12, 16, 18, 26-27 of landing-report.md)
```
- Command: `jj bookmark create go-skill-vb-ib8i-sub9 -r @ && jj git push --bookmark go-skill-vb-ib8i-sub9`
- Result: pushed bookmark `go-skill-vb-ib8i-sub9` to `origin`.
```
The command is listed but the stdout/stderr of the actual execution is not. "Result" is a claim. **Downgraded to UNSUPPORTED.**

### vb-8cw4 (line 12 of landing-report.md)
```
- Pushed: git push -u origin polecat/vb-8cw4
```
No raw output. **Downgraded to UNSUPPORTED.**

## Raw evidence attached
No raw landing push or bead sync logs were found in the repository. Searched:
- `.beads/` directory (all landing reports, STATE.md files)
- `.evidence/` directory
- `evidence/` directory
- `reports/` directory
- All `.log`, `.txt`, and `.jsonl` files under evidence paths

**Result: 0 raw evidence files found.**

## Files changed
- Created: `.beads/vb-mo87c/evidence-bundle.md` (this file)
- Created: `.beads/vb-mo87c/STATE.md`
- Created: `.beads/vb-mo87c/research-notes.md`
- Created: `.beads/vb-mo87c/routing-ledger.jsonl`
- Created: `.beads/vb-mo87c/verification-ledger.jsonl`
- Created: `.beads/vb-mo87c/implementation.md`

## Impact
This audit confirms that 50 landing claims across 48 beads and 2 combined evidence files lack raw push/sync evidence. These claims are downgraded from their current STATUS (LANDED/APPROVED/SUCCESS) to UNSUPPORTED pending re-execution with raw log capture.

## Related beads
- vb-lik4k (EPIC): Evidence Landing — stale unsupported claims cleanup
- vb-5kow2: Evidence: remove unsupported PASS claims without raw logs
- vb-awdgy: Evidence: reconcile contradictory Kani baseline reports
