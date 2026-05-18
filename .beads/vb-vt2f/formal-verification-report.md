# Formal Verification Report — vb-vt2f State 11 attempt 6

STATUS: APPROVED

## Startup rule citation

- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md`; lines 21-24 require approved formal plan, every obligation accounted, scope-before-status, and fail-closed missing required tools/evidence; lines 100-114 require exact-command execution, output classification, TLA/Kani lane discipline, and no silent waivers.
- Read `/home/lewis/.agents/skills/formal-verifier/SKILL.md`; same controlling content observed, and the agents copy wins on conflict.

## Inputs

- Workdir: `/home/lewis/src/bd-vb-vt2f-bdd` only.
- Manifest: `.beads/vb-vt2f/dispatch-state11-formal-verifier-attempt6.json`.
- Approved reviews present: `contract-verification-review.md`, `proof-review.md`, `test-review.md`, `test-plan-review.md`, `test-suite-review.md`.
- Trace-independent repair input: `.beads/vb-vt2f/implementation.md` records shard-owned `terminal_runs` tombstones replacing trace-retention stale detection.
- JSONL gate: `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl` parsed with `jq`.

## Commands run

Environment for cargo/moon commands: `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0`.

1. `rtk ls "/home/lewis/src/bd-vb-vt2f-bdd" && mkdir -p "/home/lewis/src/bd-vb-vt2f-bdd/.tmp" && pwd -P && test -s .beads/vb-vt2f/proof-obligations.jsonl && test -s .beads/vb-vt2f/proof-obligations.planned.jsonl && test -s .beads/vb-vt2f/traceability-matrix.jsonl && test -s .beads/vb-vt2f/delivery-scope.jsonl && test -s .beads/vb-vt2f/baseline-report.md && test -s .beads/vb-vt2f/tla-spec.md && test -s .beads/vb-vt2f/lean-contract.md && test -s .beads/vb-vt2f/contract-verification-review.md && test -s .beads/vb-vt2f/test-review.md && test -s .beads/vb-vt2f/proof-review.md && rtk grep -n '^STATUS: APPROVED$' ... && rtk grep -n '^PUBLIC_SURFACE_AUDIT: PASS$' ... && jq -c . ...` → PASS; cwd `/home/lewis/src/bd-vb-vt2f-bdd`; required approvals and public-surface audit present.
2. `cargo nextest run -p velvet-ballastics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted` → PASS; run ID `70fb5f9e-b06c-47e7-80d5-2ceae3eb3a5c`; `1 test run: 1 passed, 13 skipped`.
3. `cargo nextest run -p velvet-ballastics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` → PASS; run ID `f215647f-0ab3-4d4e-ad9a-dd35ee52a382`; `14 tests run: 14 passed, 0 skipped`.
4. `cargo nextest run -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog` → PASS; run ID `b996c7a3-e5d8-4951-b4cb-6685c04fa5a1`; `13 tests run: 13 passed, 0 skipped`.
5. `rtk cargo test -p vb_runtime answer_ask --all-features` → PASS; `1 passed, 1531 filtered out`.
6. `rtk cargo test -p vb_runtime --all-features` → PASS; `1532 passed (10 suites, 21.69s)`.
7. `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/bd-vb-vt2f-bdd/.tmp TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp tlc -config verification/tla/Vt2fRuntimeLifecycle.cfg verification/tla/Vt2fRuntimeLifecycle.tla` → PASS; `3600 states generated, 1302 distinct states found`; no error.
8. `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=/home/lewis/src/bd-vb-vt2f-bdd/.tmp TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp tlc -config verification/tla/Vt2fStrictAdmission.cfg verification/tla/Vt2fStrictAdmission.tla` → PASS; `2892 states generated, 1096 distinct states found`; no error.
9. `cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics` → PASS; raw `/home/lewis/.local/share/opencode/tool-output/tool_e3c49a7a80015neZZrVtEANPqy`; `0 of 489 failed`; `7 of 7 cover properties satisfied`; `VERIFICATION:- SUCCESSFUL`.
10. `cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics` → PASS; raw `/home/lewis/.local/share/opencode/tool-output/tool_e3c4a275d001x3LY8XL11P013T`; `0 of 122 failed`; `8 of 8 cover properties satisfied`; `VERIFICATION:- SUCCESSFUL`.
11. `moon ci; rc=$?; printf 'MOON_CI_EXIT=%s\n' "$rc"; exit "$rc"` → PASS; raw `/home/lewis/.local/share/opencode/tool-output/tool_e3c4e9cf8001AzrDsx9ke49onI`; `9016 tests run: 9016 passed (1 slow), 2 skipped`; `Tasks: 20 completed (4 cached)`; `MOON_CI_EXIT=0`.

## Obligation classification

- PASS: 35
- WAIVED: 5
- FAIL_LOCAL: 0
- FAIL_REGRESSION: 0
- DEFERRED_GLOBAL: 0

## Waivers

- `WAIVER-TLA-VT2F-001`: WAIVED as superseded audit row only; active lifecycle TLA row passed.
- `WAIVER-TLA-VT2F-002`: WAIVED as superseded audit row only; active strict-admission TLA row passed.
- `WAIVER-VERUS-VT2F-001`: WAIVED as superseded audit row only; not approval evidence.
- `WAIVER-LEAN-VT2F-001`: WAIVED; theorem-kernel scope not applicable per `lean-contract.md`.
- `WAIVER-VERUS-VT2F-002`: WAIVED/APPROVED for `vb-vt2f` only by `.beads/vb-vt2f/proof-review.md`; approval records non-vacuum Verus infeasibility without production refactor and explicit non-reuse/expiry caveats.

## Residual risk

- Kani rows remain owner-authorized projection-kernel evidence only; `PROJ-EQ-VT2F-001` manual review accepts this trusted-boundary risk for `vb-vt2f` only and not as executable concrete-runtime equivalence.
- Trace-retention LETHAL-001 is specifically covered by the new trace-eviction public BDD and full direct API target after shard-owned terminal tombstones.

## Decision

All 40 planned obligations are accounted in `.beads/vb-vt2f/verification-ledger.jsonl`. Required local/release obligations are `PASS` or approved `WAIVED`; no failure or deferred-global debt remains.

Next route: State 12 black-hat-reviewer rerun.
