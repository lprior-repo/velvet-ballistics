bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 7
updated_at: 2026-05-09T00:00:00Z

# Manual QA Smoke Report

## Target
- Binary: `velvet-ballastics` (debug build)
- Interface: CLI subcommand `cancel`
- Test environment: /tmp/cancel-qa-db*

## Interface Surface
```
cancel <run_id> --db <path> [--reason <text>] [--json|--jsonl]
```

## Test Matrix

| ID | Category | Command | Expected | Actual | Status |
|----|----------|---------|----------|--------|--------|
| 1 | Happy Path | cancel 999 --db /tmp/empty | Success, idempotent | "Run 999 cancelled (run not found, idempotent)" | PASS |
| 2 | Happy Path | cancel 999 --db /tmp/empty --json | JSON success | `{"success":true,"status":"cancelled","run_id":"999"}` | PASS |
| 3 | Happy Path | cancel 1 --db /tmp/finished --reason "x" --json | Idempotent on finished | `{"success":true,"note":"run already terminal"}` | PASS |
| 4 | Missing Input | cancel 42 (no --db) | Error: missing --db | "missing argument: --db" + help | PASS |
| 5 | Invalid Input | cancel abc --db /tmp/db | Error: invalid run_id | "invalid run_id 'abc': invalid digit found in string" | PASS |
| 6 | Boundary | cancel 1 --db /tmp/db --reason 257-chars | Error: too long | "reason exceeds maximum length of 256 characters" | PASS |
| 7 | Output Format | cancel 1 --db /tmp/db --jsonl | JSONL output | `{"success":true,...}` single line | PASS |

## Findings

### OBSERVATION: Pre-existing `submit` command has Fjall lock bug
- **Severity**: OBSERVATION (pre-existing, not introduced by this bead)
- **Description**: `velvet-ballastics submit` opens the Fjall journal twice in the same process, causing `FjallError: Locked`
- **Reproduction**: `submit workflow.yaml --input-bin input.bin --db /tmp/db --durability strict`
- **Evidence**: `error opening journal at /tmp/cancel-qa-db4: fjall journal operation failed: FjallError: Locked`
- **Impact**: Cannot test cancel on a live submitted run via CLI submit. Workaround: integration tests seed journal directly.

## Summary
- Total tests: 7
- PASS: 7
- FAIL: 0
- Pre-existing issues: 1 (submit lock bug)

STATUS: PASS
