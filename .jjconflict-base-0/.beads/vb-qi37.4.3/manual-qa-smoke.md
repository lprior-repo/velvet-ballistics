bead_id: vb-qi37.4.3
bead_title: runtime/storage: Persist run header before acknowledgement
phase: State 7 - manual QA smoke
updated_at: 2026-05-11T00:00:00Z

STATUS: PASS

## Invocation
- Command: `rtk cargo test -p vb_runtime submit_direct_returns_durability_error_before_ack_when_header_cannot_persist`
- Workdir: `/home/lewis/src/Velvet-ballistics-vb-qi37-4-3-go`
- Output: `cargo test: 1 passed, 1346 filtered out (2 suites, 0.01s)`

## Result
The real runtime test verifies failed before-ack persistence prevents `submit_direct` success acknowledgement.
