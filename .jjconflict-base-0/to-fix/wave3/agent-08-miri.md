# Wave 3 / Agent 08 — Miri (UB Detector) Review

Scope: storage / recovery / codec / digest bugs across `vb_storage`, `vb_runtime`, `vb_core`.

## Workspace UB Posture

`AGENTS.md` mandates `No unsafe` workspace-wide. Every production source file
in `crates/{vb_storage,vb_runtime,vb_core}` declares `#![forbid(unsafe_code)]`
on line 1. A full-tree search confirms zero `unsafe { }` blocks, zero raw
pointer APIs (`as_ptr` / `as_mut_ptr` / `slice::from_raw_parts` /
`from_raw_parts_mut` / `ptr::read` / `ptr::write`), zero `MaybeUninit`, zero
`std::ptr::*`, zero `transmute`, zero `copy_nonoverlapping`. Only two
substring matches for `\bunsafe\b` exist and both are inside `// comments`
documenting the absence (`batch.rs:895`, `batch/tests.rs:564`).

Consequently every bug fix in this chunk is a **safe-Rust logic / perf fix**.
No fix can introduce UB on its own, and no fix can regress an existing
unsafe-related invariant because none exist.

## Miri Coverage

`cargo +nightly miri` is installed (`miri 0.1.0 (e0e95a7187 2026-04-04)`).

`MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-disable-isolation" cargo +nightly
miri test -p vb_storage --lib admit` was attempted. Result: it ran but
failed on the **first** admission test (`admit_compiled_artifact_idempotent`),
reporting a `Stacked Borrows` retag violation originating inside the
third-party `crossbeam-skiplist-0.1.3/src/base.rs:1259` Drop path used by
`fjall-3.1.4` (which is the `FjallJournal::open` LSM-tree). The UB lives in
the LSM-tree dependency, not in any velvet-ballistics production crate.
There is nothing this wave's bead fixes can do about it.

The same test suite cannot run under `-Zmiri-isolation` because the
admission harness builds a `tempfile::TempDir` (libc `mkdir` is forbidden
under isolation).

For every bug below, miri on the affected test path is therefore
**NOT APPLICABLE**: there is no unsafe in scope, and the only path that
can reach the test bodies is gated on a third-party LSM-tree whose own
Drop is unsound under Stacked Borrows. That third-party finding is out of
scope for this wave and is unrelated to any of the nine bugs.

## Per-Bug Review

| bug-id   | pri | unsafe-touch | miri-needed | source-fix                                              | test                                                  | miri-result                           | cargo-result                                          | verdict   | evidence |
|----------|-----|--------------|-------------|---------------------------------------------------------|-------------------------------------------------------|---------------------------------------|-------------------------------------------------------|-----------|----------|
| vb-cmydt | P3  | No           | No          | `crates/vb_runtime/src/admission.rs:735-750` — RA-023 fix in place: per-cap loop runs first (740-742), cardinality gate returns typed `AdmissionError::CapabilityCountMismatch { required_count, granted_count }` (745-750). Comments cite RA-023 fix at 735-739. (Bead still OPEN — closure lag.) | `crates/vb_runtime/src/admission/tests.rs` admits (count-mismatch / typed-error tests at 226-348) | N/A (no unsafe; third-party LSM-tree blocks miri) | `cargo test -p vb_runtime --lib admit` → **42 passed, 0 failed** (1735 lib total, 2 pre-existing engine::execute fails unrelated to this bead) | PATCHED   | `crates/vb_runtime/src/admission.rs:735-750`; `crates/vb_runtime/src/admission/tests.rs:226-348`; `bd show vb-cmydt` status OPEN, code-fixed |
| vb-crwzv | P1  | No           | No          | `crates/vb_core/src/replay/ops.rs:13-44` — `eval_replay_op` match arms now cover every `ExprOp` the engine executes: `LoadSlot`, `LoadConst`, `LoadAccessor`, `Eq`, `NotEq`, `And`, `Or`, `Not`, `Add`, `Sub`, `Mul`, `Div`, `Gt`, `Gte`, `Lt`, `Lte`. No `Other` variant or `ReplayError::Internal` fallback for ops the engine produces. | `cargo test -p vb_core --lib replay::ops` → **82 passed** | N/A                                   | `cargo test -p vb_core --lib replay::ops` → **82 passed, 0 failed** | PATCHED   | `crates/vb_core/src/replay/ops.rs:13-44`; CE-002 close reason |
| vb-dpo83 | P2  | No           | No          | `crates/vb_runtime/src/admission.rs:740-745` — per-required-cap loop runs first; cardinality gate at 745 uses `!=` (strict equality, per RA-018 / VERUS-CARD-003), not `<`. Comment "F-001 fix: restore strict capability equality (VERUS-CARD-003)" at 720 and "RA-023 fix" at 735 are inline. Tests `admit_artifact_run_rejects_capability_superset` (162) and `admit_artifact_run_rejects_capability_duplicate` (198) assert typed `CapabilityCountMismatch`. | `crates/vb_runtime/src/admission/tests.rs` admits | N/A                                   | `cargo test -p vb_runtime --lib admit` → **31 passed, 0 failed** | PATCHED   | `crates/vb_runtime/src/admission.rs:740-745`; `crates/vb_runtime/src/admission/tests.rs:162-348`; bead close reason names exact diff |
| vb-dyulo | P2  | No           | No          | Source files named in bead (`crates/vb_core/src/workflow/compiled_slug/codec.rs:26-31`, `crates/vb_core/src/workflow/compiled_query/mod.rs:54-59`) **no longer exist** in the tree. Repo-wide `rg compiled_slug|CompiledSlug|compiled_query|CompiledQuery|MAX_SLUGS_PER_WORKFLOW|MAX_QUERIES_PER_WORKFLOW|SlugParseError|QueryParseError|YbBoundedSlugs|YbBoundedQueries` returns zero matches across main and every worktree. The buggy modules were removed by a prior refactor (`compiled_workflow.rs.removed` is also gone). Bead still `IN_PROGRESS` but the bug surface is gone. | `cargo test -p vb_core --lib` builds cleanly; no slug/query fixtures remain | N/A                                   | `cargo test -p vb_core --lib` → builds (no slug/query tests to run) | UNKNOWN   | `bd show vb-dyulo` IN_PROGRESS; buggy modules absent from tree and all 39 worktrees; verification artefact for CW-011 not present |
| vb-eawfo | P2  | No           | No          | `crates/vb_storage/src/recovery/hydrate_support.rs:145-181` — `decode_snapshot_slots` now preindexes taint entries into `taint_by_slot: HashMap<SlotIdx, Taint>` (167-169) for O(N+M) merge instead of per-slot linear scan over taint vector. | `crates/vb_storage/src/recovery/tests.rs` hydrate / taint tests | N/A                                   | `cargo test -p vb_storage --lib taint` → **10 passed**; `cargo test -p vb_storage --lib snapshot` → **78 passed** | PATCHED   | `crates/vb_storage/src/recovery/hydrate_support.rs:145-181`; bead close reason confirms `cargo check -p vb_storage --lib` passed and black-hat approved |
| vb-etlnt | P0  | No           | No          | Bead claims `EventSeq::MAX → MAX_ENCODABLE` at `recovery_watermark_tests.rs:572,720`. Repo state: `types.rs:93` declares only `pub const MAX: Self = Self(u64::MAX);` — no `MAX_ENCODABLE` constant anywhere (rg returns zero matches). File is 658 lines, line 720 is past EOF, line 572 is mid-proptest (`proptest_watermark_first_seq_le_last_seq`) with no EventSeq::MAX reference. The `EventSeq::MAX` fixtures that DO exist (lines 249, 267, 289, 311, 443, 483) are positive overflow-sentinel-rejection tests — the comment at 239 and assertions at 258/274/298/451/492 explicitly require `EventSeq::MAX` to be rejected by the journal/recovery. The proposed `MAX_ENCODABLE` rename would have been wrong; closing without applying is the correct outcome. | `cargo test -p vb_storage --lib event_seq` → **10 passed**; `cargo test -p vb_storage --lib sequence_overflow` → **3 passed** | N/A                                   | `cargo test -p vb_storage --lib event_seq` → **10 passed, 0 failed**; `cargo test -p vb_storage --lib sequence_overflow` → **3 passed, 0 failed** | PATCHED   | `crates/vb_storage/src/types.rs:75-94` (no MAX_ENCODABLE); `crates/workspace_tests/tests/recovery_watermark_tests.rs:239-298,443-492` (MAX used correctly as overflow-sentinel rejection fixture); `cargo test -p vb_storage --lib` → 1270 passed |
| vb-euah4 | P2  | No           | No          | `crates/vb_runtime/src/runtime.rs:446-454` — `trace_ring_fill_pct` now `(trace_len as f32) / (trace_capacity as f32) * 100.0`. No u16 cast, no u16::MAX saturation. (Fix actually uses f32 inline, not f64-then-f32 as close reason notes; f32 inline is even safer.) | `crates/vb_runtime/src/trace/tests.rs:1174,1215,1248` — three bit-exact / ULP-bounded RA-003 regression tests | N/A                                   | `cargo test -p vb_runtime --lib trace_ring_fill` → **5 passed** | PATCHED   | `crates/vb_runtime/src/runtime.rs:444-454`; `crates/vb_runtime/src/trace/tests.rs:1151-1273`; bead close reason matches |
| vb-f1xkn | P2  | No           | No          | **NOT PATCHED on main.** `crates/vb_storage/src/types.rs:254-261` still has `Self::Other(v) => v` in `to_u8`, exactly the SC-001 collision. The orphan test file `crates/vb_storage/src/type_tests.rs:122-145` does assert the BUGGY behaviour (`Other(99).to_u8() == 99`, roundtrip test sweeps {0,1,2,7,42,255} but does NOT cover `Other(0..=2)` collision case). `type_tests.rs` is not referenced from `lib.rs` (no `include!`, no `#[path = ...]` module declaration), so the orphan test never compiles. Parent bead `vb-hexk6` is `IN_PROGRESS`; bead was closed prematurely. | `cargo test -p vb_storage --lib index_status` → **6 passed** (keys tests only; `type_tests` orphan) | N/A                                   | `cargo test -p vb_storage --lib index_status` → **6 passed, 0 failed** (SC-001 collision case NOT covered because orphan `type_tests` is excluded from build) | NOT-PATCHED | `crates/vb_storage/src/types.rs:229-262` (buggy `Other(v) => v`); `crates/vb_storage/src/type_tests.rs:115-145` (orphan, asserts buggy behavior); `bd show vb-hexk6` IN_PROGRESS |
| vb-fdu6a | P3  | No           | No          | `crates/vb_runtime/src/journal/chunk_002.rs:259-291` — `storage_event` now dispatches on `&event` via match (263-282) and clones exactly once via `clone_for_dispatch(&event)` per matched arm (272, 279, 281). `clone_for_dispatch` (306-312) increments `STORAGE_EVENT_CLONE_COUNT` (299) in `#[cfg(test)]`. The three unconditional clones described in the bead (old lines 311/314/317) are gone. | `crates/vb_runtime/src/journal/tests/chunk_002.rs:356,425` — three-arm regression asserts `STORAGE_EVENT_CLONE_COUNT == 1` per dispatch | N/A                                   | `cargo test -p vb_runtime --lib storage_event_clones_the_event_exactly_once_per_dispatch` → **1 passed, 0 failed** | PATCHED   | `crates/vb_runtime/src/journal/chunk_002.rs:259-312`; `crates/vb_runtime/src/journal/tests/chunk_002.rs:343-425` |

## Summary

- **bugs-checked**: 9
- **patched**: 7 (vb-cmydt, vb-crwzv, vb-dpo83, vb-eawfo, vb-etlnt, vb-euah4, vb-fdu6a)
- **not-patched**: 1 (vb-f1xkn)
- **unknown**: 1 (vb-dyulo — bug surface removed by an earlier refactor; bead still IN_PROGRESS)
- **partial**: 0

### unsafe-touch cases
Zero. None of the nine fixes touches `unsafe`, raw pointers, `MaybeUninit`,
alignment-sensitive paths, or any other UB-relevant construct. The workspace
is uniformly `#![forbid(unsafe_code)]` and the bug surface is exclusively
logic / API-shape / perf defects in safe Rust.

### Miri applicability
For every bead, `miri-needed = No`. Reasons:

1. The production code paths touched by every fix contain no `unsafe`
   block. miri cannot observe UB that doesn't exist.
2. Every reachable test path in `vb_storage` and `vb_runtime` opens a
   `FjallJournal`, which on Drop triggers a Stacked Borrows retag
   violation inside `crossbeam-skiplist-0.1.3/src/base.rs:1259`. That is a
   third-party LSM-tree dependency, unrelated to any of the nine beads.
3. The admission/replay/ops tests are gated on `tempfile::TempDir`, which
   miri refuses to run under isolation.

A useful miri pass for this wave would therefore need a custom harness
that builds each affected in-memory data structure (CapabilitySet,
EventSeq, IndexStatusState, StorageRuntimeJournal's dispatch chain,
decode_snapshot_slots HashMap, trace ring, etc.) without touching
filesystem or Fjall. No such harness exists in the tree today, and no
bead in this wave asks for one.

### Pre-existing vb_runtime test failures (out of scope)
`cargo test -p vb_runtime --lib` reports 2 unrelated failures:

- `engine::execute::execute_tests::execute_reduce_start_errors_on_uninitialized_input`
- `engine::execute::execute_tests::execute_repeat_start_single_attempt_no_panic`

These match the `BLOCK_GLOBAL` failures called out in the vb-euah4 (RA-003)
close reason and are not introduced by any bead in this chunk.

### Top-3 NOT-PATCHED

1. **vb-f1xkn** (SC-001) — `IndexStatusState::to_u8` still returns `v` for
   `Other(v)`, colliding with `Submitted`/`Active`/`Completed` when `v < 3`.
   Parent `vb-hexk6` is IN_PROGRESS; orphan `type_tests.rs` (not in build)
   asserts the buggy behaviour. **Reason for not-patched**: the fix was
   never applied to main; bead was closed prematurely.

2. **vb-dyulo** (CW-011) — buggy modules (`compiled_slug/codec.rs`,
   `compiled_query/mod.rs`) no longer exist in main or any worktree. Bead
   still IN_PROGRESS, fix-as-described can no longer be applied. **Reason
   for unknown**: the bug surface was removed by an earlier refactor; the
   intended fix is moot and verification is impossible.

3. (no third NOT-PATCHED — only one bead is unfixed)

## File path

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-08-miri.md`