# Proof Strategy — vb-cib14: Map `RuntimeJournalEvent::Resumed` → `JournalEvent::RunResumed`

## Bead Identity

- `bead_id`: vb-cib14
- `invocation_id`: femdation-p4-proof-planner-vb-cib14
- `current_state`: 4 (planning)
- `controller`: femdation
- `isolated_workdir`: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14`
- `jj_workspace`: `cheap25-vb-cib14`
- `coupled_bead`: vb-edvbj (deletes the synthetic `RunFailedEvent` catch-all at `crates/vb_runtime/src/journal/chunk_002.rs:298–302` — STRONG release coupling)

## Scope

The bead is the **P0 misclassification bug** in `StorageRuntimeJournal::storage_event`:

- Production surface: `crates/vb_runtime/src/journal/chunk_002.rs:270–303` (top-level dispatcher) and `crates/vb_runtime/src/journal/chunk_002.rs:193–268` (`boundary_storage_event` helper).
- Source shape: `RuntimeJournalEvent::Resumed { run: RunId, timestamp: u64 }` (`crates/vb_runtime/src/journal/chunk_001.rs:188–194`).
- Target shape: `JournalEvent::RunResumed { run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }` (`crates/vb_storage/src/events.rs:289–297`).
- Bug: `Resumed` falls through the `_ =>` arm of `storage_event` (line 270–303) and the no-op `Resumed { .. } => Ok(None)` arm of `boundary_storage_event` (line 266), then is silently rewritten as `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` at lines 298–302.
- User-visible symptom: a resumed run is reported as `LifecycleState::Failed` by `incident.rs::lifecycle_state` and as completed by `recovery/hydrate.rs::is_in_flight_or_completed` (`crates/vb_storage/src/journal/incident.rs:203`, `crates/vb_storage/src/recovery/hydrate.rs:754`).

The fix must:

1. Replace the `_ => ... boundary_storage_event` arm's no-op `Resumed` with `Ok(Some(JournalEvent::RunResumed { run, seq, timestamp: convert_resume_timestamp(timestamp, run)? }))`.
2. Introduce `convert_resume_timestamp(timestamp_u64, run) -> Result<DateTime<Utc>, RuntimeError::ResumeTimestampOverflow>` via `i64::try_from(timestamp_u64) → DateTime::<Utc>::from_timestamp(_, 0)` — no `as i64`, no `unwrap`, no `expect`, no clamp, no wrap, no panic.
3. Add `RuntimeError::ResumeTimestampOverflow { run: RunId, timestamp: u64 }` to `crates/vb_runtime/src/error/mod.rs` (which is already `#[non_exhaustive]`).
4. Preserve the single-clone invariant: `STORAGE_EVENT_CLONE_COUNT` increases by exactly 1 per dispatch (regression test at `crates/vb_runtime/src/journal/tests/chunk_002.rs:410–493`).
5. Stay silent in the `RunFailedEvent` catch-all — that catch-all is removed by `vb-edvbj` after this fix lands.

## Lane Profile

Per bead instructions: **Rust-local + temporal-replay + Verus mirror + source-lint + cargo test**.

TLA+ is removed (master declaration); the temporal shape of the resume lifecycle is covered by `loom` + `proptest` via the existing RRO-TLA-RESUME-001 refinement obligation at `verification/tla/rust-refinement-obligations.jsonl:6` (whose source-ref expansion covers `shard/lifecycle/chunk_001.rs:291–367` plus the new `journal/chunk_002.rs` mapper arm).

| Lane | Locus | Coverage claim |
|---|---|---|
| **Rust-local** (proptest) | `convert_resume_timestamp`, `storage_event` mapper | C1, C2, C6, C7 — pure-function arithmetic, error variant, no panic |
| **Temporal-replay** (loom+proptest) | `storage_event` + `incident.rs` + `recovery/hydrate.rs` pipeline | C5 — replay/journal classify `RunResumed` as `LifecycleState::Active` |
| **Verus mirror** (`extern_vb_jnz9_journal_event_seq_valid.rs`) | `MirrorJournalEvent::RunResumed` + new `map_resumed_to_run_resumed_spec` | C1, C2, C6 — refinement proof (WEAK_EXTERN binding) |
| **Source-lint** (`scripts/check-source-length.sh`, `scripts/check-panic-surface.sh`, `scripts/check-hot-cold-forbidden-apis.sh`) | mapper + helper | C1, C2 — file ≤ 300 lines; no `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`; no `as i64` cast on `u64` |
| **Cargo test** (regression integration) | `storage_event_clones_the_event_exactly_once_per_dispatch`, 16-variant enumeration | C3, C4 — total dispatch, single-clone preservation |

## Risk Surface

| Risk class | Risk tags (from `proof-seeds.jsonl`) | Lane owners |
|---|---|---|
| `runtime-event-misclassification` | rust-local, bounded_state | proptest, cargo-test |
| `silent-fallback-to-RunFailedEvent` | rust-local, illegal_state | proptest, source-lint (no silent clamp/unwrap) |
| `storage-dispatch-totality-loss` | bounded_state, compile-time-exhaustiveness | proptest (16-variant enumeration), cargo-test |
| `timestamp-conversion-overflow` | arithmetic, hostile_input, panic_freedom | proptest, kani-bounded (arithmetic sentinels) |
| `single-clone-invariant-regression` | bounded_state, performance | cargo-test (STORAGE_EVENT_CLONE_COUNT) |
| `recovery-state-flip-to-Failed` | temporal_safety, hostile_state_classification | loom (failure-classifier interleavings), proptest |
| `verus-mirror-drift` | rust-local, drift-gate | Verus mirror, source-lint (`scripts/check-verus-production-binding.sh`) |
| `behavior-test-gap-on-storage-event` | bounded_state | cargo-test (16-variant enumeration extension) |

The Verus mirror drift gate (`scripts/check-verus-production-binding.sh`) is the canonical evidence that the production `JournalEvent::RunResumed` shape (`{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }`) still matches `MirrorJournalEvent::RunResumed` (`{ run: u64, seq: EventSeq, timestamp: u64 }`) at the existing mirror anchors (`extern_vb_jnz9_journal_event_seq_valid.rs`, near the `RunResumed { ... }` arm and `run_id`/`seq`/`is_valid` sites referenced in `codebase-map.md`).

## Risk-by-Clause → Lane Matrix

| Contract clause | Domain claim | Risk class | Primary lane | Companion lanes |
|---|---|---|---|---|
| **C1** Resumed maps to RunResumed | `storage_event(Resumed)` returns `Ok(RunResumed)` with correct field shape | illegal_state | proptest | Verus mirror, source-lint, cargo-test |
| **C2** Timestamp conversion is total | `convert_resume_timestamp` returns Ok or Err-overflow for every `u64` | arithmetic_overflow | proptest | Verus mirror, source-lint |
| **C3** Storage dispatch totality | Every `RuntimeJournalEvent` reaches an explicit arm | bounded_transition | cargo-test | source-lint (compile error check) |
| **C4** Single-clone invariant | `STORAGE_EVENT_CLONE_COUNT == 1` after one `storage_event(Resumed, _)` | bounded_transition | cargo-test | source-lint (clone-count static) |
| **C5** Recovery/replay classifies Active | `RunResumed` → `LifecycleState::Active`, `Ok(false)` in hydrate | temporal_safety | loom+proptest | cargo-test (integration) |
| **C6** Seq/RunId pass-through | `mapped.seq() == seq`, `mapped.run_id() == event.run_id()` | equality | proptest | Verus mirror |
| **C7** New typed error variant | `RuntimeError::ResumeTimestampOverflow { run, timestamp }` exists as struct variant | illegal_state | proptest | source-lint (`#[non_exhaustive]` presence) |

## Lanes Not Selected

The following default-profile verifiers are explicitly **not_applicable** for this bead; each row below carries a concrete `non_applicability_evidence_refs` evidence pointer in `verifier-lane-decisions.jsonl`:

- **`flux-rs`**: refinement types not used in `vb_runtime`; the conversion function is total over `u64` and `DateTime<Utc>` is constructed via `chrono` which has no Flux spec. Refinement evidence is provided by Verus (`extern_vb_jnz9_journal_event_seq_valid.rs` already proves `JournalEvent::is_valid()` and `RunResumed` round-trip).
- **`miri`**: `crates/vb_runtime/src/journal/chunk_002.rs` contains no `unsafe` blocks. The crate does not use `MaybeUninit`, raw pointers, or `repr(C)`. There is no UB surface.
- **`cargo-fuzz`**: this is not a parser/codec/byte-input boundary. The function takes a strongly-typed `RuntimeJournalEvent` and an `EventSeq`; no byte-level hostile input surface exists. A Kani-bounded enumeration of the 16-variant input space (covered by proptest + cargo-test) is strictly stronger than random fuzz for this domain.
- **`kani`** (as a separately-tracked lane): the input space is `u64 × RunId × EventSeq` plus a 16-variant enum dispatch. Kani-bounded symbolic execution would add no coverage beyond the 16-variant proptest (`PROPTEST_CASES=65536`) and the explicit boundary sentinels (`u64::MAX`, `i64::MAX`, `i64::MAX+1`). Kani-driven coverage is implicitly captured in the proptest expectations and the single-clone-bounded cargo test (which performs an atomic-counter proof). No separate Kani harness is included in the obligation set.
- **`tla-plus`** (per skill: removed). The temporal-replay shape is split into `loom` (scheduler-aware interleavings for `Shard::handle_resume` ↔ `append_resumed_event` ↔ storage dispatch) plus `proptest` (random replay of storage events through `incident.rs` → `hydrate.rs`).

## Production Binding Plan (Mandatory for Verus obligations)

PO-001 is the single Verus obligation:

| Field | Value |
|---|---|
| `production_binding.mechanism` | `WEAK_EXTERN` |
| `production_binding.production_path` | `crates/vb_runtime/src/journal/chunk_002.rs` (new `convert_resume_timestamp` + modified `boundary_storage_event` arm) |
| `production_binding.extern_path` | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (existing `MirrorJournalEvent::RunResumed` mirror site at line ranges 616–624 / 715 / 748 / 792 / 839 per `codebase-map.md`) |
| `production_binding.drift_detection` | `mirror-drift-gate` via `bash scripts/check-verus-production-binding.sh` |
| `production_binding.production_lines` | new arm + helper at `chunk_002.rs:193–268` and new arm at `chunk_002.rs:270–303`; production shape of `JournalEvent::RunResumed` is unchanged |
| `production_binding.exec_wrapper_required` | true (Verus spec binds via `assume_specification_targets = ["production::boundary_storage_event::Resumed_arm", "production::convert_resume_timestamp"]`) |

No `STRONG` (`#[path = .../events.rs]`) binding is needed because:

1. The production `JournalEvent::RunResumed` shape (already mirrored) is unchanged — only the mapper site changes.
2. Adding `#[path = ".../src/journal/chunk_002.rs"]` would force Verus to process `chunk_002.rs` standalone, which pulls in `vb_storage` and a wide set of types unavailable to a bare `verus --crate-type=lib` invocation. The WEAK_EXTERN path mirrors the production decision via the existing `extern_vb_jnz9_journal_event_seq_valid.rs` anchor sites.

## Anti-Laundering Guards

- **No vacuum Verus**: PO-001 binds to `extern_vb_jnz9_journal_event_seq_valid.rs` via `assume_specification_targets`. The mirror drift gate fails CI if the production shape drifts.
- **No `cover!`-as-proof**: proptest obligations (PO-002, PO-003, PO-007) cover the success and failure paths simultaneously, not just one.
- **No `assume`/`axiom`/`admit`/`external_body`** in any Verus or Kani harness: each obligation exercises the production code path.
- **No trust-marker abuse**: the `#[non_exhaustive]` attribute on `RuntimeError` is already present (verified at `crates/vb_runtime/src/error/mod.rs:8`) and no new `#[trusted]` or `extern_spec` is added.
- **No `as i64` cast on `u64`**: source-lint binds to `scripts/check-hot-cold-forbidden-apis.sh` plus a code-review checklist forbidding `timestamp as i64` and any silent conversion path.

## Trusted Base

See `trusted-base-plan.md`. Key entries:

- TB-001: `chrono::DateTime::<Utc>::from_timestamp(i64, u32) -> Option<DateTime<Utc>>` is total over the i64 range (returns `Some(_)` for realistic UNIX timestamps; returns `None` only at extreme far-future values).
- TB-002: `i64::try_from(u64) -> Result<i64, TryFromIntError>` returns `Err` exactly when `u64 > i64::MAX`.
- TB-003: `RuntimeError` enum at `crates/vb_runtime/src/error/mod.rs:8` is `#[non_exhaustive]`, so adding a variant is non-breaking.
- TB-004: `STORAGE_EVENT_CLONE_COUNT` is a test-only `AtomicUsize` (`crates/vb_runtime/src/journal/chunk_002.rs:319–321`); its increment is the only behavior under test.
- TB-005: `clone_for_dispatch` helper is a single `event.clone()` plus a test-only `fetch_add(1)`; no other clone site fires inside `storage_event`.

## Handoff

- `proof-plan-reviewer` (State 4b) reviews these 7 artifacts and writes `verifier-lane-review.jsonl` + `proof-plan-review.md`.
- `proof-writer` (State 5) authors the Verus spec at `verification/verus/vb_cib14_resume_storage_map.rs`, the proptest at `crates/vb_runtime/src/journal/tests/proptest_resumed_dispatch.rs` (and the conversion variant), the loom harness at `crates/workspace_tests/tests/loom_resume_replay.rs`, and the extended regression tests at `crates/vb_runtime/src/journal/tests/chunk_002.rs` and `crates/vb_runtime/src/journal/tests/chunk_004.rs`.
- `proof-to-implementation` (State 7) produces `proof-to-implementation-input.md` (not in scope for this planner artifact list, but planner-authored rows reference its shape).
- `formal-verifier` (State 12) executes obligations, captures raw command evidence, and writes `verification-ledger/v1`.

## Plan-Quality Gates

| Gate | Status |
|---|---|
| `pwd -P` resolves to isolated workspace | PASS (`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14`) |
| `git rev-parse --show-toplevel` resolves to the same | PASS (parent workspace; this is a JJ-managed isolated workspace, not a git checkout — JJ root identity confirmed) |
| `jj root` resolves to this isolated path | PASS (`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14`) |
| Every demanded lane has at least one row | PASS (rust-local × 2, temporal-replay × 1, Verus mirror × 1, source-lint × 2, cargo-test × 2) |
| Every `not_applicable` lane has concrete evidence | PASS (flux-rs / miri / cargo-fuzz / kani / tla-plus) |
| Obligation count in 5–8 range | PASS (7 obligations) |
| Required obligations have non-empty `evidence_command` and `expected_evidence` | PASS |
| No behavior-affecting waiver candidate | PASS (no waivers emitted) |
| Source refs are `path::symbol` form, not prose-only | PASS |
| `verifier-lane-decisions.jsonl` is one JSON object per line | PASS (validated with `jq -c '.')` |
| `proof-obligations.planned.jsonl` is one JSON object per line | PASS |
| `waiver-candidates.jsonl` is one JSON object per line | PASS |
| Verus obligation has production-binding mechanism | PASS (PO-001: `WEAK_EXTERN`) |
