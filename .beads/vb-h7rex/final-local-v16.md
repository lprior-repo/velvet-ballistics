# vb-h7rex Local Verification Evidence V16

Workspace: vb-h7rex-tests-v11 (isolated)
Date: 2026-10-22

## Gate Run: Final Local V16

Ran via `/tmp/vb-run-gate.py` with 600s timeout per lane. Fresh `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_*` artifacts.

Toolchain: nightly-2026-04-28
Flags: RUSTFLAGS=-Dwarnings, CARGO_BUILD_JOBS=2 (gate default)

| Lane | Command | Exit Code | Seconds | Tests Passed |
|------|---------|-----------|---------|--------------|
| 01 | `cargo test -p vb_runtime --test policy_pin_durable_recovery` | 0 | 9.909 | 5 |
| 02 | `cargo test -p vb_storage --test policy_pin_wire_contract` | 0 | 6.151 | 6 |
| 03 | `cargo fmt --check` | 0 | 1.451 | n/a (clean) |

## Totals

- **Runtime tests:** 5 passed, 0 failed
- **Storage tests:** 6 passed, 0 failed
- **Total tests:** 11 passed, 0 failed
- **Format check:** clean (no diff)
- **All gates:** passed

## Evidence Artifacts

- `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_01.json` (gate metadata)
- `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_01.stdout` (test output)
- `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_02.json` (gate metadata)
- `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_02.stdout` (test output)
- `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_03.json` (gate metadata)
- `.beads/vb-h7rex/evidence/FINAL_LOCAL_V16_03.stdout` (fmt output)
