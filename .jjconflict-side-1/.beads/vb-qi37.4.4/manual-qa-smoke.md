bead_id: vb-qi37.4.4
bead_title: runtime: Add admission durability errors
phase: State 7 - manual QA smoke
updated_at: 2026-05-11T00:00:00Z

STATUS: PASS

## Invocation
- Command: `rtk cargo test -p vb_runtime admission_header_persistence_failure_has_dedicated_diagnostic`
- Workdir: `/home/lewis/src/Velvet-ballistics-vb-qi37-4-4-go`
- Output: `cargo test: 1 passed, 1347 filtered out (2 suites, 0.00s)`

## Result
The real runtime test verifies header persistence failures use the dedicated diagnostic path.
