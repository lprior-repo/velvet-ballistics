# vb-sue37 Supply-Chain Exit-Status Verification - Contract

## Scope

Verify that the supply-chain Moon task preserves exact exit codes from all advisory and mandatory tools. The selftest executes the FULL actual script extracted from `.moon/tasks/all.yml` with PATH fixtures keyed by tool name. No production source changes. No rewritten audit block or copy of `run_advisory_report`.

## Selftest Harness

The selftest is at `.beads/vb-e7rjr/supply-chain-status-selftest.sh`. It:

1. Extracts the complete actual supply-chain script from `.moon/tasks/all.yml`
2. Creates PATH fixtures: a `rustup` shim that dispatches by tool name
3. Creates all 11 workspace package fixture directories
4. Runs the full actual script multiple times with different tool exit code profiles
5. Tests `run_advisory_report` function-level exit code capture with real stub executables
6. Uses real `timeout 1 sleep 3` for exit code 124 and actual missing binary for exit code 127

## Exit Code Preservation Requirements

### Advisory Tools (geiger per-package, deny, vet, machete)

- `run_advisory_report` captures the tool's actual exit code in `<tool>.status`
- The function always returns 0 (task continues to next report)
- Exit codes 0, 1, 2, 124, 127 must be preserved exactly in status files
- No relabeling of failures as PASS or swallowing of exit codes

### Mandatory Audit Tool

- Non-zero exit from `cargo audit` writes the status file then exits immediately with the same code
- Exit codes 1 and 3 must be propagated as task exit codes
- The task must fail-closed with the same exit code as the audit tool

## Full Script Behavioral Requirements

1. All tools exit 0 -> task exits 0, all status files show 0
2. Advisory tools fail, audit ok -> task exits 0, status files show actual codes
3. Audit exits 1 -> task exits 1 (fail-closed)
4. Audit exits 3 -> task exits 3 (fail-closed)
5. All 11 geiger status files must be created per run
6. Audit exit code must appear both in status file and as task exit code

## Evidence Requirements

- Immutable runner log at `.beads/vb-sue37/evidence/actual-extracted-script-selftest-v2.*`
- All 18 assertions must pass (PASS: 18, FAIL: 0)
- Test must exit 0 from the selftest script
- No NOTE/WARN then pass; every mismatch must exit 1

## Author Errata (vb-sue37)

The previous draft01df7ddad830 was rejected because it:
- Rewrote a copy of the audit block instead of executing the full actual script
- Only ran `run_advisory_report` as an extracted function with copied parameters
- Failed to run the full actual task with PATH fixtures and all 11 package dirs

This corrected harness executes the full actual script and tests both full-script behavior and function-level exit code capture. The two working DeepSeek harnesses at `/tmp/vb-xhrt9-extract-test.sh` and `/tmp/vb-xhrt9-extract-test2.sh` validated this approach.