# Research Notes: vb-mo87c

## Finding
Landing reports summarize git push/status/bd dolt push without raw stdout/stderr/exit logs.

## Search scope
1. All `landing-report.md` files under `.beads/`
2. Combined landing evidence files under `.beads/`
3. All `.log`, `.txt`, `.jsonl` under `evidence/`, `.evidence/`, `reports/`

## Evidence patterns checked
- `exit_code` / `exit status` / `exit_status` — none found in landing reports
- `raw.stdout` / `raw.stderr` — none found
- `--- stdout` / `--- stderr` — none found
- Full terminal output blocks — none found
- `Push complete` — appears as a claim word, not logged output
- `succeeded` — appears as an assertion, not evidence

## Key observations
1. 48 landing reports exist, all 47 with PASS/succeeded claims lack raw evidence
2. The 1 file with "exit" references (vb-qi37.14.1) contains CLI exit code contract definitions, not push command exit statuses
3. Combined landing evidence files (qi37-all, qi37-combined) list commands with PASS but no raw output
4. No raw push/sync logs exist anywhere in the evidence directory tree

## Downgrade policy applied
Per AGENTS.md GOD RULES and vb-mo87c Section 7.5 Anti-Hallucination Guard:
- If the raw evidence does not support the claim, downgrade the claim
- Claims without raw stdout/stderr/exit codes are UNSUPPORTED
- Evidence claims PASS but raw logs are missing = stale or hallucinated evidence

## Related beads
- vb-lik4k (EPIC): stale unsupported claims cleanup
- vb-5kow2: remove unsupported PASS claims without raw logs (CLOSED)
- vb-awdgy: reconcile contradictory Kani baseline reports (CLOSED)
- vb-zubeh: resolve merge conflict markers in proof evidence files (CLOSED)
