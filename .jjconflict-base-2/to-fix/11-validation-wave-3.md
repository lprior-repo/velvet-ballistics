# Wave 3 — Storage / Recovery / Codec / Digest Bug Validation

**Generated:** 2026-06-24
**Scope:** Last-week bug beads (created 2026-06-17 → 2026-06-24) touching storage/recovery/codec/digest/envelope/pending-action domain. Total: **131 bugs**.
**Method:** Read-only validation, no source mods, no beads. 15 parallel local subagents (12 core + 3 ad-hoc deep-dive).
**Pass criteria:** Source fix present + targeted cargo test passes + no Holzman regression.

## Verdict Roll-up

| Verdict | Count | % |
|---------|------:|--:|
| PATCHED | 51 | 38.9% |
| PARTIAL | 7 | 5.3% |
| NOT-PATCHED | 50 | 38.2% |
| UNKNOWN | 9 | 6.9% |
| NOT-A-BUG / Out-of-scope | 14 | 10.7% |
| **Total** | **131** | **100%** |

## Agent-by-Agent Tally

| Agent | Role | PATCHED | PARTIAL | NOT-PATCHED | UNKNOWN | Other |
|-------|------|--------:|--------:|------------:|--------:|------:|
| 00 | holzman-rust A | 2 | 2 | 5 | 0 | 0 |
| 01 | holzman-rust B | 0 | 0 | 7 | 0 | 0 |
| 02 | explore | 0 | 2 | 5 | 0 | 0 |
| 03 | black-hat | 3 | 0 | 4 | 0 | 0 |
| 04 | truth-serum | 4 | 0 | 4 | 0 | 0 |
| 05 | flux-rs | 3 | 0 | 5 | 1 | 0 |
| 06 | arch-drift | 7 | 1 | 1 | 1 | 0 |
| 07 | test-reviewer | 3 | 3 | 2 | 1 | 0 |
| 08 | miri | 7 | 0 | 1 | 1 | 0 |
| 09 | verus | 4 | 1 | 4 | 1 | 0 |
| 10 | hands-on-qa | 5 | 0 | 4 | 0 | 0 |
| 11 | rust-contract | 5 | 0 | 2 | 2 | 0 |
| 12 | ad-hoc: storage-codec | 5 | 2 | 1 | 1 | 1 |
| 13 | ad-hoc: recovery-hydration | 2 | 0 | 7 | 0 | 0 |
| 14 | ad-hoc: digest-binding | 4 | 1 | 0 | 4 | 0 |
| **Totals** | | **54** | **14** | **52** | **12** | **2** |

## Major Phantom Closures

| Bead | Cited symbol | Reality |
|------|--------------|---------|
| vb-1rqz7.33 (SR-014) | `recovery_stamps.rs`, `put_recovery_stamp` | File deleted from main; function doesn't exist |
| vb-1rqz7.32 (SR-004) | `SnapshotMissing` variant | Not present; `load_snapshot` conflated with `CorruptSnapshot` |
| vb-1rqz7.4 (SR-001) | `events_for_run_full` | Symbol does not exist; `recover_full_journal:203` still uses snapshot-tail |
| vb-1rqz7.20 | `BatchBuilder::try_push` API | Doesn't exist; `push` still infallible/unbounded |
| vb-2eprq (SA-002) | `JournalError::BatchAborted` | Variant doesn't exist; `batch.rs:324-330` still returns `Ok(())` |
| vb-1rqz7.21 | `verify_content_digest` on compiled_ir, `metadata_hash` field | Path bypassed; field doesn't exist |
| vb-1rqz7.23 | Kani harness `minimal_valid_workflow()` | Hardcodes structural shape (GOD RULE 1 violation) |
| vb-1rqz7.1 (SJ-002) | `RecordKind::SequenceGap=60`, `MAGIC_JOURNAL_SEQUENCE_GAP` | Don't exist; `injection.rs:47` still uses `RunCancelled` |
| vb-1rqz7.2 (SJ-003) | `inject_raw_event`, `inject_seq_gap` `write_lock`/`contains_key` dedup | Lacking |
| vb-2ljzq (SR-004) | Same `SnapshotMissing` distinction | Still conflated |
| vb-9bxs9 | `recovery_stamp` keyspace | Entirely deleted |
| vb-byd2v | function rename `_unchecked_len` | Still has unchecked prefix on main |
| vb-dyulo | buggy modules | No longer exist in tree or any worktree |

## Tests That Pin the Bug

| Bead | Test pinning |
|------|--------------|
| vb-1rqz7.10 (SR-008) | `summary/tests.rs:329-332` asserts `Ok(())` on missing-RunAccepted |
| vb-1rqz7.14 (SC-002) | `run_event_key_with_max_values` asserts `EventSeq::MAX` encodes |
| vb-1rqz7.20 | `batch_append_event_allows_duplicate_key_insertion` enshrines dup insertion |
| vb-83aqs (SA-002) | `batch.rs:1860` enshrines `Ok(())` on abort |
| vb-9gjzb (RP-011 from W1) | `collect_start_uses_source_as_collector_when_output_is_none_for_non_empty` |
| vb-hxul3 (CV-105 from W1) | `proptest_registry_consistency.rs:68-69` asserts `0x13` collision |

## Wildcard Lifecycle Arms (carried + new in Wave 3)

| Location | Risk |
|----------|------|
| `vb_storage/src/journal/incident.rs:168` | `_ => LifecycleState::Active` (covered by `#[allow(unreachable_patterns)]`) — vb-1rqz7.3, vb-7gm7c |
| `vb_storage/src/recovery/replay/summary.rs:550` | `_ => Ok(self)` for `apply_frame_event` (catches 10 non-seed-affecting variants) |
| `vb_storage/src/recovery/replay/summary.rs:223` | `_ =>` for summary-event-checked fallback |
| `vb_storage/src/recovery/hydrate_support.rs:236` | `_ =>` for dimension derivation fallback |
| `vb_storage/src/recovery/replay/summary.rs:86` | `RunResumed | RunRetried | RunAnswered => {}` (max_slot never updated) |

## Digest-Binding Violations

| Violation | Location | Effect |
|-----------|----------|--------|
| `put_compiled_ir` direct path skips digest verify | `journal/source.rs:47` | Storage accepts forged IR under arbitrary digest key |
| `put_compiled_ir` batch path skips digest verify | `batch.rs:109` | Same |
| `verify_digests` `Full` variant omits ABI + policy | `recovery/recover.rs:83-101` | Section 18 §8 only partially enforced |
| Constant-time compare: array `==` | `codec/payload.rs:13` | Not exploitable (digest is trusted on-disk) but not constant-time |

## Pending Action Hydration Gaps

- Summary path at `summary.rs:111-152` never publishes `pending_actions`
- `FrameSeedAccumulator` (`summary.rs:401-460`) lacks explicit "action abandoned" event
- Cancellation resolving an outstanding schedule leaves stale pending action

## Drift Status

| File | Lines | Status |
|------|------:|--------|
| `batch.rs` | 2005 | over-300 (drift) |
| `recovery/replay/summary.rs` | 999 | over-300 |
| `types.rs` | 606 | over-300 |
| `admission.rs` | 540 | over-300 |
| `hydrate.rs` | 536 | over-300 |
| `hydrate_support.rs` | 484 | over-300 |
| `journal/incident.rs` | 412 | over-300 |
| `trimming/logic.rs` | 383 | over-300 |

Function drift: `trim_events_for_run` 45 lines (+20); `derive_lifecycle_state_from_events` 26 lines (+1).

## Workspace Blockers (carry-over)

| Blocker | Location | Effect |
|---------|----------|--------|
| Duplicate function | `crates/vb_runtime/src/test_harness.rs:33-58/63-88` `iterator_state_in_slot` | Blocks vb_runtime lib tests |
| Malformed test file | `crates/vb_storage/src/preview.rs:42-154` | Blocks storage lib tests |
| Unresolved merge markers | `crates/vb_runtime/src/shard/types.rs:807-815` | Blocks vb_runtime --tests |
| Dead test file | `crates/vb_runtime/src/engine/drive_tests.rs` (1269 lines) | RE-001 dead code |
| Orphan Kani modules | `verification/kani/` 9 of 13 modules unwired | Kani harnesses not exercised |

## Test Suite Status (full sweep, not gated)

| Suite | Result |
|-------|--------|
| `vb_storage --lib` | 1270 passed / 0 failed |
| `vb_runtime --lib` | 1734 passed / 0 failed (2 unrelated panics on `engine/execute` StepIdx) |
| `vb_core --lib` | 2142 passed / 0 failed |
| `vb_validate --lib` | 836 passed / 0 failed |
| `workspace_tests/vb_qi37_4_2` | 22 passed / 0 failed |
| `vb_replay` | 3 passed |

## Holzman / NASA-JPL Findings

- **No new Holzman violations introduced** by any PATCHED path
- All production crates declare `#![forbid(unsafe_code)]` at lib root
- 0 unsafe-touch cases (storage domain is pure safe Rust)
- miri-skipped: not reachable (`forbid(unsafe_code)`). 1 miri run reached `crossbeam-skiplist-0.1.3` Drop — unrelated Stacked Borrows retag in third-party dep
- Dominant failure mode: **silent fallthrough** (returns `Ok`/`?`-propagated success where contract requires typed error)

## Per-Agent Reports

- `to-fix/wave3/agent-00-holzman-rust-A.md`
- `to-fix/wave3/agent-01-holzman-rust-B.md`
- `to-fix/wave3/agent-02-explore.md`
- `to-fix/wave3/agent-03-black-hat.md`
- `to-fix/wave3/agent-04-truth-serum.md`
- `to-fix/wave3/agent-05-flux-rs.md`
- `to-fix/wave3/agent-06-arch-drift.md`
- `to-fix/wave3/agent-07-test-reviewer.md`
- `to-fix/wave3/agent-08-miri.md`
- `to-fix/wave3/agent-09-verus.md`
- `to-fix/wave3/agent-10-hands-on-qa.md`
- `to-fix/wave3/agent-11-rust-contract.md`
- `to-fix/wave3/agent-12-adhoc-storage-codec.md`
- `to-fix/wave3/agent-13-adhoc-recovery-hydration.md`
- `to-fix/wave3/agent-14-adhoc-digest-binding.md`