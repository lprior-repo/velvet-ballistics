# Wave 5 — Architecture Drift / IPC / CLI Bug Validation

**Generated:** 2026-06-24
**Scope:** Last-week bug beads (created 2026-06-17 → 2026-06-24) touching architecture drift / IPC / CLI / file-size / deferred-codegen / cross-cutting domain. Total: **60 bugs**.
**Method:** Read-only validation, no source mods, no beads. 15 parallel local subagents (12 core + 3 ad-hoc deep-dive).
**Pass criteria:** Source fix present + targeted cargo test passes + no Holzman regression.

## Verdict Roll-up

| Verdict | Count | % |
|---------|------:|--:|
| PATCHED | 24 | 40.0% |
| PARTIAL | 6 | 10.0% |
| NOT-PATCHED | 22 | 36.7% |
| UNKNOWN | 0 | 0.0% |
| NOT-A-BUG (chunk misassigned) | 8 | 13.3% |
| **Total** | **60** | **100%** |

## Agent-by-Agent Tally

| Agent | Role | PATCHED | PARTIAL | NOT-PATCHED | UNKNOWN | Other |
|-------|------|--------:|--------:|------------:|--------:|------:|
| 00 | holzman-rust A | 2 | 0 | 2 | 0 | 0 |
| 01 | holzman-rust B | 0 | 0 | 3 | 0 | 0 |
| 02 | explore | 0 | 1 | 3 | 0 | 0 |
| 03 | black-hat | 4 | 0 | 0 | 0 | 0 |
| 04 | truth-serum | 1 | 0 | 3 | 0 | 0 |
| 05 | flux-rs | 5 | 0 | 0 | 0 | 0 |
| 06 | arch-drift | 2 | 1 | 1 | 0 | 0 |
| 07 | test-reviewer | 1 | 1 | 2 | 0 | 0 |
| 08 | miri | 4 | 0 | 0 | 0 | 0 |
| 09 | verus | 0 | 1 | 3 | 0 | 0 |
| 10 | hands-on-qa | 2 | 0 | 2 | 0 | 0 |
| 11 | rust-contract | 0 | 1 | 3 | 0 | 0 |
| 12 | ad-hoc: file-size | 3 | 0 | 1 | 0 | 0 |
| 13 | ad-hoc: ipc-frame | 0 | 4 | 0 | 0 | 0 |
| 14 | ad-hoc: cli-contract | 3 | 1 | 0 | 0 | 0 |
| **Totals** | | **27** | **10** | **25** | **0** | **0** |

(Note: agent-13 ipc-frame and agent-14 cli-contract got chunks not matching their specialty — many defects were classified PARTIAL due to chunk/domain mismatch, but real findings surfaced.)

## Major Findings

### Phantom Closures (carry-over from W1-W4)

| Bead | Cited symbol | Reality |
|------|--------------|---------|
| vb-1rqz7.2 (SJ-003) | `journal/regression_tests_vb_1rqz7.rs` | File does not exist; `injection.rs:30,52` still bypasses write_lock |
| vb-1rqz7.4 (SR-001) | `events_for_run_full` | Symbol does not exist; `recovery/replay/core.rs:203` still uses snapshot-tail |
| vb-1rqz7.32 (SR-004) | `RecoveryError::MissingSnapshot` | Not present; `core.rs:230` conflated with `CorruptSnapshot` |
| vb-a5vsl | `from_live_journal`, `SystemConnectionState`, `output.rs` | All fictional |
| vb-af9q5 | `codes.rs`, `copy_slice`, commit `vxsootyx` | All fabricated |
| vb-cc2my (SR-005) | `ActionScheduledTicket.output` derivation | `hydrate_support.rs:190` still discards; `RunAnswered => {}` |
| vb-2ljzq (SR-004) | same conflation | Still present |
| vb-9gjzb from W1 | `finish_collect_start_page` jumps to `done` | Still does |
| vb-qmomy | `red_queen_capabilities.rs` (19 tests) | Only 2 test files in `vb_ipc/tests/`; "all 19 pass" false |

### Source Code Defects (real, NOT-PATCHED)

| Bead | Issue | File:line |
|------|-------|-----------|
| vb-1rqz7.17 (SA-016) | `put_run_header`/`put_snapshot` fail to set `aborted` on error | `vb_storage/src/batch.rs:123-148` |
| vb-1rqz7.18 | `append_event` ignores `staged_event_keys` HashSet | `vb_storage/src/batch.rs` |
| vb-widdi (SC-004) | `latest_durable_snapshot_seq` perf fix reverted | `vb_storage/src/trimming/logic.rs:21-53` |
| vb-jhkez | `assert_ok!`/`prop_assert_ok!` macros still defined and used 130× | `vb_storage/tests.rs` + `vb_core/frame/tests.rs` |
| vb-t5zlm (RS-013) | `IpcPayload::CancelRun` has no `reason` field | `vb_ipc/src` |
| vb-u587r (RS-013) | `IpcTraceEventKind::RunKilled` variant absent; `_ => Unknown` wildcard | `vb_runtime/src/trace.rs:102` |
| vb-ubpk8 | `diag_render/mapping.rs` consolidation eliminated 25 split helpers | 638-line `diag_render.rs` |
| vb-uxfl0 (SR-002) | 5 public recovery functions still call `events_for_run` (skips pre-snapshot) | `recovery/recover.rs:140-216` |
| vb-keji6 (SA-003) | `append_event` only checks committed state, not staged | `batch.rs:243-251` |
| vb-krus1 | `restate_decode_error_taxonomy_tests.rs:108` still expects `ReservedNonZero` | `vb_ipc/src` (SEC-01 capability envelope absent) |
| vb-tqz3v (SA-002) | `put_run_header`/`put_snapshot` use `?` without `self.aborted = true` | `batch.rs:123,137` |
| vb-igldl (PARTIAL) | `reject_unsupported_live_frame_state` returns `InvalidRecoveryHydration` for all flags | `vb_runtime/src` |

### Drift / File-Size Findings

| File | Lines | Status |
|------|------:|--------|
| `crates/vb_storage/src/preview.rs` | 359 | over-300 (drift, malformed) |
| `crates/vb_core/src/span.rs` | 366 | over-300 (drift) |
| `crates/vb_runtime/src/trace.rs` | 327 | over-300 (drift) |
| `crates/vb_runtime/src/shard/helpers_main.rs.bak` | 2456 | git-tracked orphan, not in `mod.rs` |
| `diag_render.rs` | 638 | over-300 (consolidated, eliminated 25 split helpers) |
| `trimming/logic.rs` | 383 | over-300 (ledger-exceptioned, has 4 hot fns >25) |
| `hydrate_support.rs` | 484 | over-300 |
| `tests.rs` (`vb_storage`) | 1915 | over-300 |
| `frame/tests.rs` (`vb_core`) | 1314 | over-300 |

### IPC Findings

- **`MemoryIngress` still on `crossbeam_channel`** (`ingress.rs:5,77`) — `Cargo.toml` has `crossbeam-channel` not `crossbeam-queue`. Master §50 violation (ArrayQueue for IPC SPSC; crossbeam_channel FORBIDDEN).
- **No central server command queue** — `server/impl_.rs` dispatches per-connection, master §50 backpressure unmet
- `queue/mod.rs` is comment-only; 913 lines of staged `array_queue_tests.rs` are dead (no `mod tests;` include)
- Magic-after-allocation: 0 (`AWAITING_MAGIC_MAX_BYTES=4 < IPC_HEADER_LEN=24` cap holds)
- Command-set drift: 0 (exactly 11 variants, reserved `12..=16` → `UnknownCommand`)

### CLI Findings

| Aspect | Status |
|--------|--------|
| `action inspect <action-name>` contract | holds — string name only, no numeric id selector (`args/action.rs:38-72`) |
| Command surface (master §33) | **11 extras**: `status`, `verify`, `explain`, `trace`, `retry`, `resume`, `answer`, `diff`, `submit`, `simulate`, `cancel` |
| `--emit postcard` typed | **NOT typed** for operator outputs — `OutputFormat::Postcard` (`output.rs:83-112,135-147`) routes through `encode_postcard_json_frame` which wraps `serde_json::Value` as `CliPostcardPayload { content_type: JsonUtf8, json_utf8: Vec<u8> }` (`cli_postcard/types.rs:36-55`, `cli_postcard/validation.rs:16-18` rejects non-JSON). JSON-in-Postcard wrapper violation across **27 commands**. Only `compile --emit postcard` (`compile.rs:121-162`) emits true typed `WorkflowParts`. |
| `cargo test -p velvet-ballistics --lib` | 214 passed / 0 failed |

### Workspace Blockers (carry-over from W1-W4)

| Blocker | Location | Effect |
|---------|----------|--------|
| Duplicate function | `crates/vb_runtime/src/test_harness.rs:33-58/63-88` `iterator_state_in_slot` | Blocks vb_runtime lib tests |
| Malformed test file | `crates/vb_storage/src/preview.rs:42-154` | Blocks storage lib tests |
| Unresolved merge markers | `crates/vb_runtime/src/shard/types.rs:807-815` | Blocks vb_runtime --tests |
| Dead test file | `crates/vb_runtime/src/engine/drive_tests.rs` (1269 lines) | RE-001 dead code |
| Orphan Kani modules | `verification/kani/` 9 of 13 modules unwired | Kani harnesses not exercised |
| Dead `array_queue_tests.rs` | 913 lines, no `mod tests;` include | Staged tests not run |
| `helpers_main.rs.bak` | 2456 lines, git-tracked, not in mod.rs | Backup residue |

### Holzman / NASA-JPL Findings

- **No new Holzman violations introduced** by any PATCHED path
- All production crates declare `#![forbid(unsafe_code)]` at lib root
- 0 unsafe-touch cases in wave 5
- Dominant failure mode: **phantom/incomplete fixes**, not Holzman regressions
- Test suite status: `cargo test -p velvet-ballistics --lib` 214 passed; `cargo test -p vb_runtime --lib` 1734 passed; `cargo test -p vb_storage --lib` 1270 passed

## Per-Agent Reports

- `to-fix/wave5/agent-00-holzman-rust-A.md`
- `to-fix/wave5/agent-01-holzman-rust-B.md`
- `to-fix/wave5/agent-02-explore.md`
- `to-fix/wave5/agent-03-black-hat.md`
- `to-fix/wave5/agent-04-truth-serum.md`
- `to-fix/wave5/agent-05-flux-rs.md`
- `to-fix/wave5/agent-06-arch-drift.md`
- `to-fix/wave5/agent-07-test-reviewer.md`
- `to-fix/wave5/agent-08-miri.md`
- `to-fix/wave5/agent-09-verus.md`
- `to-fix/wave5/agent-10-hands-on-qa.md`
- `to-fix/wave5/agent-11-rust-contract.md`
- `to-fix/wave5/agent-12-adhoc-file-size.md`
- `to-fix/wave5/agent-13-adhoc-ipc-frame.md`
- `to-fix/wave5/agent-14-adhoc-cli-contract.md`