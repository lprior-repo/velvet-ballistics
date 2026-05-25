# vb-vt2f State 11 machine-gate report

bead_id: vb-vt2f
phase: 11
attempt: 6
STATUS: PASS

## Attempt 6 current machine gate after trace-independent stale ask repair

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`.

Environment: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0`.

- `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted` -> PASS; run ID `70fb5f9e-b06c-47e7-80d5-2ceae3eb3a5c`; `1 test run: 1 passed, 13 skipped`.
- `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` -> PASS; run ID `f215647f-0ab3-4d4e-ad9a-dd35ee52a382`; `14 tests run: 14 passed, 0 skipped`.
- `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog` -> PASS; run ID `b996c7a3-e5d8-4951-b4cb-6685c04fa5a1`; `13 tests run: 13 passed, 0 skipped`.
- `rtk cargo test -p vb_runtime answer_ask --all-features` -> PASS; `1 passed, 1531 filtered out`.
- `rtk cargo test -p vb_runtime --all-features` -> PASS; `1532 passed (10 suites, 21.69s)`.
- `moon ci; rc=$?; printf 'MOON_CI_EXIT=%s\n' "$rc"; exit "$rc"` -> PASS; raw `/home/lewis/.local/share/opencode/tool-output/tool_e3c4e9cf8001AzrDsx9ke49onI`; `9016 tests run: 9016 passed (1 slow), 2 skipped`; `Tasks: 20 completed (4 cached)`; `MOON_CI_EXIT=0`.

Machine gates are green after shard-owned terminal tombstones replaced trace-retention stale ask detection.
