# Wave 5 — Agent 11: Rust Contract (Type/Domain Auditor)

**Date:** 2026-06-24
**Bug chunk:** `/tmp/wave5-chunk-11.txt` (4 IDs: `vb-tw2jd`, `vb-u587r`, `vb-ubpk8`, `vb-uxfl0`)
**Working dir:** `/home/lewis/src/velvet-ballistics` (verified `git rev-parse --show-toplevel`)
**Domain:** typestates, domain invariants, Section-17 typed errors, IPC/CLI/storage boundaries

## Method

For each ID: read bead (`bd show`), locate claimed fix path in source, run a
targeted `cargo test` for the touched crate, and grade against the close
reason. The contract lens is per-bead: (a) typestate preserved for any
state-machine surface (command parser, IPC mapping), (b) domain invariant
intact (workspace shape, file-size drift, recovery correctness), (c) error
taxonomy matches Section 17 of `velvet-ballistics-MASTER.md`.

## Cross-cutting findings

- All four beads are CLOSED, but source-level inspection shows the
  claimed fixes have not been applied. Three of four are NOT-PATCHED
  on source (vb-u587r, vb-ubpk8, vb-uxfl0); one is PARTIAL because the
  test target the bead cites has been deleted but the underlying
  workspace-shape invariant is still violated (vb-tw2jd).
- `vb-u587r` and `vb-uxfl0` were independently flagged NOT-PATCHED by
  wave1/agent-12 (`to-fix/wave1/agent-12-adhoc-yaml-grammar.md:57,62`)
  and wave2/agent-13 (`to-fix/wave2/agent-13-adhoc-journal-replay.md:59`)
  on prior HEAD. The same defects are still present on this HEAD.
- `scripts/check-source-length.sh` (referenced by the master contract's
  `source-length` gate) still reports three production files over 300
  physical lines with no valid exception row:
  - `crates/vb_core/src/span.rs` (366 lines)
  - `crates/vb_runtime/src/trace.rs` (327 lines)
  - `crates/vb_storage/src/preview.rs` (359 lines)

  Plus 7 stale exception entries for files now under the 300-line limit
  (e.g. `crates/vb_cli/src/app_impl.rs` 75 lines, `payloads.rs` 144
  lines). Stale entries are kept non-fatal by the script, but the
  ledger is still dirty.

## Per-bug findings

| bug-id | pri | source-fix | test | typestate | invariant | error-taxonomy | targeted-cmd | result | verdict | evidence |
|--------|-----|------------|------|-----------|-----------|----------------|--------------|--------|---------|----------|
| vb-tw2jd | P1 | bead cites `crates/workspace_tests/tests/vb_a0t1_source_length_gate_tests.rs:134` — that test target **does not exist** in tree (`workspace_tests` package has 60+ test targets, none matching `*a0t1*`); `vb_a0t1_source_length_gate_tests.rs` returns "no test target named" from cargo | `cargo test -p velvet-ballistics-workspace-tests --test vb_a0t1_source_length_gate_tests test_full_source_length_pipeline` | n/a (test target deleted) | BROKEN — `check-source-length.sh` still reports 3 production files (`span.rs:366`, `trace.rs:327`, `preview.rs:359`) over 300 physical lines with no valid `.config/source-length-exceptions.txt` row, plus 7 stale exception entries and 2 malformed rows at end of ledger (lines 483-484) | n/a | `bash scripts/check-source-length.sh` | reports over-limit files; `cargo test ... vb_a0t1_source_length_gate_tests` → error: no such test target | PARTIAL | `check-source-length.sh` exit output; `payloads.rs` line count 144 (under limit but still in exception ledger line 141); `.config/source-length-exceptions.txt:483-484` malformed; `vb_core/src/span.rs:366`, `vb_runtime/src/trace.rs:327`, `vb_storage/src/preview.rs:359` |
| vb-u587r | P0 | `crates/vb_ipc/src/payloads.rs:101-140` `IpcTraceEventKind` enum has **no `RunKilled` variant**; `crates/vb_ipc/src/server/trace.rs:102` still ends with `_ => IpcTraceEventKind::Unknown` — `TraceEvent::RunKilled` (defined at `vb_runtime/src/trace.rs:280-283`) falls through to `Unknown`; the 3 tests claimed in close reason (`trace_event_kind_maps_run_killed`, `run_killed_roundtrip_via_postcard`, `ipc_trace_event_kind_roundtrip_run_killed`) do not exist | `cargo test -p vb_ipc --lib RunKilled` and `--lib trace_event_kind_maps_run_killed` both filter 0 tests | BROKEN — IPC `TraceEvent -> IpcTraceEventKind` mapping is partial (wildcard `_ => Unknown` violates the total-function contract for the IPC event surface; equivalent to a non-exhaustive typestate on the wire protocol) | INTACT — magic check (`vb_ipc/src/server/helpers.rs:33-45`) and command-set wiring (11 variants) unchanged; the bug is local to the event-kind enum + mapping | BROKEN — adding `RunKilled` variant was the explicit fix; the enum's `#[non_exhaustive]` (payloads.rs:100) is correct but the variant is missing, so `Unknown` is silently returned for terminal kill evidence (violates Section 17 / 21 typed-error surface) | `cargo test -p vb_ipc --lib trace_event` | 22 passed (no `RunKilled` coverage); `RunKilled` filter → 0 tests | NOT-PATCHED | `vb_ipc/src/payloads.rs:101-140` (no `RunKilled` arm); `vb_ipc/src/server/trace.rs:102` wildcard `_ => IpcTraceEventKind::Unknown`; `vb_runtime/src/trace.rs:280-283` defines `TraceEvent::RunKilled { run }` |
| vb-ubpk8 | P1 | bead cites `crates/vb_validate/src/diag_render/mapping.rs` (758 lines); that file **does not exist** — the entire `diag_render/` directory has been replaced by a single `crates/vb_validate/src/diag_render.rs` (638 lines); `map_schema_*` and `map_validation_*` helpers do not exist anywhere in `vb_validate`; the per-family submodules promised in the close reason (`mapping/schema.rs`, `mapping/reference.rs`, `mapping/control_flow.rs`, `mapping/type_taint_resource.rs`, `mapping/gate.rs`, `mapping/contract.rs`) do not exist | `cargo test -p vb_validate --lib diag_render` | UNKNOWN — the typestate that the bead targeted (per-family submodule split) is no longer present; the surviving `diag_render.rs` is a flat 638-line file still over 300, so the architectural-drift invariant is still violated, just in a different file | BROKEN — `diag_render.rs` is 638 physical lines (>300 cap); 17 `map_schema_*` + 8 `map_validation_*` helpers were either removed or inlined; closure criterion (per-family split) not met; no replacement split | n/a (no error path) | `cargo test -p vb_validate --lib diag_render --no-fail-fast` | 31 passed (render_tests pass on consolidated code, but the architectural-drift split fix is absent) | NOT-PATCHED | `crates/vb_validate/src/diag_render.rs` (638 lines, single file); no `crates/vb_validate/src/diag_render/` directory; `grep -r 'map_schema_\|map_validation_' crates/vb_validate` → 0 matches |
| vb-uxfl0 | P1 | `crates/vb_storage/src/recovery/recover.rs:144,160,199,211,228` — all four public recovery functions (`recover_runtime_summary`, `recover_runtime_summary_with_expected`, `recover_runtime_frame_seed`, `recover_run_admission`) and `recover_all_incomplete_runs` still call `journal.events_for_run(run)`; `crates/vb_storage/src/journal/replay.rs:72-85` `events_for_run_bounded` still skips events at or before `latest_durable_snapshot_seq(run)`; no `events_for_run_full` reader exists; no typed-error variant for snapshot-presence rejection (`RecoveryError` enum at `recovery/types.rs:39-129` has 12 variants, none for snapshot-handling) | `cargo test -p vb_storage --lib recover_runtime` | BROKEN — public recovery API contract is silent: when a `RunSnapshot` exists, callers receive a summary built only from the post-snapshot tail (`events_for_run` skips at-or-before `latest_durable_snapshot_seq`), dropping `RunAccepted`/`RunAdmission`/step states/slot writes/action schedules from the prefix. SR-002 invariant violation | BROKEN — the bug's invariant "summary includes prefix events" is not enforced; the only readers that touch snapshots are `recovery/hydrate.rs:115-149` (input-validation layer), not the public recover.rs APIs | BROKEN — the fix required either an explicit rejection with a typed error or a merge reader; neither path exists. Closest variant `RecoveryError::NoRecoveryData { run }` is the wrong semantics (it means "no events at all", not "events truncated by snapshot"); `RecoveryError::SnapshotNotHandled` / `RecoveryError::SnapshotTruncatedPrefix` / similar are absent | `cargo test -p vb_storage --lib 'recovery::tests' --no-fail-fast` | 102 passed; but `cargo test -p vb_storage --lib events_for_run_full` → 0 tests (no symbol); `pre_snapshot` snapshot-tail tests confirm the *current* (buggy) skip behavior | NOT-PATCHED | `recover.rs:144,160,199,211,228` call `events_for_run`; `journal/replay.rs:77-83` skip at-or-before snapshot; `journal/tests.rs:1759 fn events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed` confirms skip is intentional; no `events_for_run_full` in `journal/replay.rs` or `journal/readonly.rs`; `RecoveryError` enum at `recovery/types.rs:39-129` lacks snapshot-handling variant |

## Targeted test runs (verbatim tail)

```
$ cargo test -p vb_ipc --lib trace_event --no-fail-fast
running 22 tests
test server::trace::tests::ipc_trace_event_kind_roundtrip_action_scheduled ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_action_failed ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_action_completed ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_run_cancelled ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_step_ended ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_run_finished ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_ask_answered ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_step_started ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_run_failed ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_run_submitted ... ok
test server::trace::tests::ipc_trace_event_kind_roundtrip_slot_written ... ok
test server::trace::tests::trace_event_kind_maps_action_completed ... ok
test server::trace::tests::trace_event_kind_maps_action_failed ... ok
test server::trace::tests::trace_event_kind_maps_action_scheduled ... ok
test server::trace::tests::trace_event_kind_maps_ask_answered ... ok
test server::trace::tests::trace_event_kind_maps_run_cancelled ... ok
test server::trace::tests::trace_event_kind_maps_run_failed ... ok
test server::trace::tests::trace_event_kind_maps_run_finished ... ok
test server::trace::tests::trace_event_kind_maps_run_submitted ... ok
test server::trace::tests::trace_event_kind_maps_slot_written ... ok
test server::trace::tests::trace_event_kind_maps_step_ended ... ok
test server::trace::tests::trace_event_kind_maps_step_started ... ok
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 518 filtered out

$ cargo test -p vb_ipc --lib RunKilled --no-fail-fast
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 540 filtered out

$ cargo test -p vb_storage --lib 'recovery::tests' --no-fail-fast
running 102 tests
... (all 102 pass)
test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 1171 filtered out

$ cargo test -p vb_storage --lib events_for_run_full --no-fail-fast
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1273 filtered out

$ cargo test -p vb_storage --lib pre_snapshot --no-fail-fast
running 3 tests
test recovery::tests::hydrate_run_frame_tests::hydrate_run_frame_applies_tail_completion_without_pre_snapshot_schedule ... ok
test journal::tests::events_for_run_starts_after_snapshot_when_pre_snapshot_trimmed ... ok
test journal::tests::events_for_run_skips_corrupt_pre_snapshot_event_by_key_range ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1270 filtered out

$ cargo test -p vb_validate --lib diag_render --no-fail-fast
running 31 tests
... (all 31 render_tests pass on the consolidated file)
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 805 filtered out

$ cargo test -p velvet-ballistics-workspace-tests --test vb_a0t1_source_length_gate_tests --no-fail-fast
error: no test target named `vb_a0t1_source_length_gate_tests` in `velvet-ballistics-workspace-tests` package

$ bash scripts/check-source-length.sh
.config/source-length-exceptions.txt:22 stale exception for crates/vb_cli/src/app_impl.rs with 75 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:27 stale exception for crates/vb_cli/src/args/workflow.rs with 192 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:28 stale exception for crates/vb_cli/src/cli_postcard.rs with 42 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:141 stale exception for crates/vb_ipc/src/payloads.rs with 144 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:145 stale exception for crates/vb_ipc/src/server/handlers/event.rs with 2 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:422 stale exception for crates/vb_ipc/src/server/handlers/tests.rs with 3 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:423 stale exception for crates/vb_ipc/src/server/handlers_tests.rs with 2 physical lines (limit >300); keeping non-fatal
.config/source-length-exceptions.txt:483 malformed row; expected <file_path>|<owner>|<split_bead>|<removal_plan>|<reason>
.config/source-length-exceptions.txt:484 malformed row; expected <file_path>|<owner>|<split_bead>|<removal_plan>|<reason>
crates/vb_core/src/span.rs has 366 physical lines (limit <=300) and no valid .config/source-length-exceptions.txt row
crates/vb_runtime/src/trace.rs has 327 physical lines (limit <=300) and no valid .config/source-length-exceptions.txt row
crates/vb_storage/src/preview.rs has 359 physical lines (limit <=300) and no valid .config/source-length-exceptions.txt row
```

## Summary

- **Bugs checked:** 4
- **Verdicts:** NOT-PATCHED × 3 (vb-u587r, vb-ubpk8, vb-uxfl0); PARTIAL × 1 (vb-tw2jd)
- **Typestate-broken cases:** 3 — vb-u587r (IPC mapping partial via wildcard `_`), vb-uxfl0 (recovery API contract silent on snapshot-presence), vb-tw2jd (workspace shape invariant violated: test target deleted, three production files over-limit)
- **Error-taxonomy mismatches:** 2 — vb-u587r (silent `Unknown` return for `RunKilled` instead of typed variant), vb-uxfl0 (no `RecoveryError` variant for snapshot-handling; closest existing `NoRecoveryData` has wrong semantics)
- **Top-3 NOT-PATCHED with one-line reasons:**
  1. **vb-u587r** — `IpcTraceEventKind` enum lacks `RunKilled` variant and the mapping at `trace.rs:102` still falls through `_ => IpcTraceEventKind::Unknown` for the existing `TraceEvent::RunKilled` event.
  2. **vb-uxfl0** — `recover_runtime_summary`, `recover_runtime_summary_with_expected`, `recover_runtime_frame_seed`, `recover_run_admission`, and `recover_all_incomplete_runs` still call `journal.events_for_run(run)` which silently skips events at-or-before `latest_durable_snapshot_seq(run)`; no `events_for_run_full` reader and no typed `SnapshotNotHandled` error variant.
  3. **vb-ubpk8** — `diag_render/mapping.rs` and the promised per-family split submodules do not exist; the surface was consolidated into a single `crates/vb_validate/src/diag_render.rs` (638 lines, still over the 300-line cap), and the 17 `map_schema_*` + 8 `map_validation_*` helpers were eliminated rather than split.
- **File path written:** `/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-11-rust-contract.md`
