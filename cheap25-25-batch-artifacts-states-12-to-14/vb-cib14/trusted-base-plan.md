# Trusted Base Plan — vb-cib14

The seven obligations in `proof-obligations.planned.jsonl` rely on the trusted surfaces listed below. Each entry is an assumption the proof explicitly or implicitly takes; each is paired with the obligation(s) that depend on it via `trusted_base_refs`.

## TB-001 — `chrono::DateTime::<Utc>::from_timestamp` is total over `i64` (with `Some`/`None` semantics)

- **Surface**: `chrono::DateTime::<Utc>::from_timestamp(secs: i64, nsecs: u32) -> Option<DateTime<Utc>>`
- **Justification**: `chrono` is a well-established dependency already used at `crates/vb_storage/src/events.rs:5` and `crates/vb_runtime/Cargo.toml:9`. The crate documents `from_timestamp`: returns `Some(_)` for `i64` values within the representable `DateTime<Utc>` range, returns `None` only for far-future values that exceed the chrono internal representation. The implementation uses standard chrono internals.
- **Scope**:
  - `secs: i64` is the result of `i64::try_from(timestamp_u64)` after the conversion gate.
  - `nsecs: 0` (fixed; not a configurable parameter).
  - `Some(_) => chrono::DateTime<Utc>` is the only success case.
- **Used by**: PO-001 (Verus spec proves totality), PO-003 (proptest conversion totality with `from_timestamp` returning `Some` for realistic UNIX timestamps), PO-004 (regression uses `from_timestamp(1_700_000_000, 0)`).
- **Risk**: a chrono major-version change could shift the representation boundary. Mitigation: `Cargo.toml` pins the chrono version via `workspace = true` and the production code uses `DateTime<Utc>` only via this single function.

## TB-002 — `i64::try_from(u64)` returns `Err` exactly when `u64 > i64::MAX`

- **Surface**: `i64::try_from(timestamp_u64: u64) -> Result<i64, TryFromIntError>`
- **Justification**: This is a primitive operation in the Rust standard library. It returns `Err` precisely when the input exceeds the destination type's maximum (`i64::MAX == 9_223_372_036_854_775_807`); for all `u64 <= i64::MAX` it returns `Ok(i64)`. No wrapping, no clamping.
- **Scope**: The mapper calls `i64::try_from(timestamp_u64)` and propagates the error via `?` after mapping to `RuntimeError::ResumeTimestampOverflow`.
- **Used by**: PO-001 (Verus spec), PO-003 (proptest conversion totality for `u64::MAX` etc.), PO-007 (proptest asserting variant-shape carrier on overflow).
- **Risk**: none at the standard-library level. The proptest does not exercise the standard library's logic; it exercises the mapper's handling of the `Err` arm.

## TB-003 — `RuntimeError` is `#[non_exhaustive]` so adding a variant is non-breaking

- **Surface**: `crates/vb_runtime/src/error/mod.rs:8` declares `#[non_exhaustive]` on `pub enum RuntimeError`. Adding `RuntimeError::ResumeTimestampOverflow { run: RunId, timestamp: u64 }` does not break any external crate that match-arms `RuntimeError`, because `#[non_exhaustive]` forbids external exhaustive matches.
- **Justification**: Verified at `crates/vb_runtime/src/error/mod.rs:8`. The lint scripts (`check-error-exhaustiveness.sh`) confirm zero non-`#[non_exhaustive]` match sites in workspace code that would break.
- **Scope**: Adding the new variant changes the public-runtime API of `vb_runtime`. Any internal exhaustive-match callers (within `vb_runtime` itself) must be updated to handle the new variant via the existing fallthrough pattern.
- **Used by**: PO-001 (Verus spec), PO-003 (proptest enumeration with the new variant), PO-006 (source-lint gate), PO-007 (proptest typed-error variant shape).
- **Risk**: an external crate could attempt an exhaustive match on `RuntimeError`. Since the enum is `#[non_exhaustive]`, the Rust compiler will reject such a match at the external crate's build time; this is a deliberate strict upgrade boundary.

## TB-004 — `STORAGE_EVENT_CLONE_COUNT` is a test-only `AtomicUsize`

- **Surface**: `crates/vb_runtime/src/journal/chunk_002.rs:319-321` declares `#[cfg(test)] static STORAGE_EVENT_CLONE_COUNT: AtomicUsize = AtomicUsize::new(0)`. Incremented inside `clone_for_dispatch` via `fetch_add(1, SeqCst)`.
- **Justification**: Atomic counter under `SeqCst` ordering is sufficient for a test invariant under a single test thread. The helper `clone_for_dispatch` is the only call site that increments the counter. The release build does not include the counter (`#[cfg(test)]`).
- **Scope**: Used by PO-004 (cargo-test single-clone regression extension). The test asserts `STORAGE_EVENT_CLONE_COUNT == 1` per dispatch.
- **Risk**: a future code change could introduce a second `clone_for_dispatch` call, inflating the counter. The test is the safety net; the source-lint gate PO-006 forbids extra clone sites via `clippy::needless_clone` deny.

## TB-005 — `incident.rs::lifecycle_state` and `recovery/hydrate.rs::is_in_flight_or_completed` are read-only classifiers

- **Surface**: `crates/vb_storage/src/journal/incident.rs:203` (`lifecycle_state(JournalEvent::RunResumed) -> LifecycleState::Active`) and `crates/vb_storage/src/recovery/hydrate.rs:754` (`is_in_flight_or_completed(JournalEvent::RunResumed) -> Ok(false)`).
- **Justification**: Both functions are pure read-only classifiers. They do not mutate any state; they classify the input event. The mapper change is upstream and these classifiers do not need to be updated — they already classify `RunResumed` correctly (verified in `codebase-map.md`).
- **Scope**: Used by PO-005 (loom+proptest replay classification). The proptest exercises both classifiers against both the production `RunResumed` event and the legacy-buggy `RunFailedEvent` rewrite.
- **Risk**: none. The classifiers are deterministic and pure.

## TB-006 — Source-length budget is 300 lines per file

- **Surface**: `scripts/check-source-length.sh` enforces the budget. `docs/rust-governance.md` defines it.
- **Justification**: Per AGENTS.md (`Engineering Rules`), source lint is zero-tolerance. The mapper change must keep `crates/vb_runtime/src/journal/chunk_002.rs` ≤ 300 lines after the new arm + helper are added. If the budget is exceeded, the implementation agent must split the helper into a companion `chunk_002_runtime_convert_timestamp.rs` file at the same level.
- **Scope**: Used by PO-006 (source-lint gate `check-source-length.sh`).
- **Risk**: if the budget is exceeded, the lint script fails. The implication is that the helper must be factored out before the new arm grows `chunk_002.rs` past 300 lines.

## TB-007 — Verus production-binding gate (`scripts/check-verus-production-binding.sh`)

- **Surface**: `scripts/check-verus-production-binding.sh` validates that every `proof fn` in `verification/verus/*.rs` is bound to production via `#[path = ".../crates/..."] mod production;` or via `extern_*.rs` companion, with at least one `assume_specification` / contract anchor.
- **Justification**: This is the canonical gate against vacuum Verus proofs. PO-001 (Verus spec at `verification/verus/vb_cib14_resume_storage_map.rs`) must declare its production-binding mechanism in `production_binding` (it does, with `WEAK_EXTERN`).
- **Scope**: Used by PO-006 (source-lint gate).
- **Risk**: if the gate fails, PO-001 cannot close. The planner already commits to WEAK_EXTERN via the existing `extern_vb_jnz9_journal_event_seq_valid.rs` mirror.

## TB-008 — Production-inner mirror drift gate (`scripts/check-production-inner-drift.sh`)

- **Surface**: `scripts/check-production-inner-drift.sh` compares `crates/*/production_inner/*` mirror files against the production source they mirror. Drift is a CI failure.
- **Justification**: The mapper site change does not introduce any new `production_inner/` mirror files. The existing `MirrorJournalEvent::RunResumed` in `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (a WEAK_EXTERN mirror, not a production_inner mirror) is bound by the production-binding gate (TB-007). Drift is therefore a TB-007 concern, not TB-008.
- **Scope**: Used by PO-006 (gate, but no production_inner mirror exists for this bead).
- **Risk**: none for this bead.

## TB-009 — Loom replayer uses single-shard, single-concurrent-task semantics

- **Surface**: The shard is single-threaded by design; no intra-shard locking. `Shard::handle_resume` → `append_resumed_event` → `storage_event` execute on the shard's owning task. The recovery classifier (`hydrate.rs`, `incident.rs`) reads from the durable journal, not from concurrent in-memory state.
- **Justification**: Verified at `crates/vb_runtime/src/shard/mod.rs` and `crates/vb_runtime/src/shard/types.rs:750-797`. The mapper is a pure function from `(event, seq)` to `JournalEvent` or `RuntimeError`.
- **Scope**: Used by PO-005 (loom+proptest temporal-replay). Loom explores scheduler interleavings for the seam between `append_resumed_event` and the recovery read; the seam is small because the recovery side is read-only and the mapper is pure.
- **Risk**: any future concurrency primitive added to the resume path would invalidate this trust; loom PO-005 is the safety net.

## TB-010 — `clone_for_dispatch` is the only `clone()` site in `storage_event`

- **Surface**: `crates/vb_runtime/src/journal/chunk_002.rs:318-324` `clone_for_dispatch(&event) -> RuntimeJournalEvent { event.clone() }`. Used inside `storage_event` dispatch arms.
- **Justification**: The mapper change must not introduce a second `clone()` site. PO-006 (source-lint gate via `clippy::needless_clone` deny + `check-source-length.sh`) and PO-004 (cargo-test single-clone) enforce this.
- **Scope**: Used by PO-001 (Verus spec assumes single-clone), PO-004 (cargo-test asserts `STORAGE_EVENT_CLONE_COUNT == 1`), PO-006 (source-lint forbids extra clones).
- **Risk**: any future clone introduced would need to update `STORAGE_EVENT_CLONE_COUNT` to reflect the new count, or the test would fail. The fix is structural, not just lint.

## TB-011 — `crates/vb_runtime` is `#[forbid(unsafe_code)]`

- **Surface**: `crates/vb_runtime/src/lib.rs:1` declares `#![forbid(unsafe_code)]`. No `unsafe` block exists in any file under `crates/vb_runtime/src/`.
- **Justification**: Verified at `crates/vb_runtime/src/lib.rs:1`. There is no UB surface for Miri (see VLD-016 `not_applicable` reason).
- **Scope**: Used by PO-006 (source-lint gates).
- **Risk**: any future `unsafe` block would require a crate-root `#![allow(unsafe_code)]`, which would be a deliberate contract regression visible at the file root.

## TB-012 — Cargo `workspace = true` pins for chrono, thiserror, postcard, atomics

- **Surface**: `crates/vb_runtime/Cargo.toml:9` (`chrono.workspace = true`), `crates/vb_storage/Cargo.toml:6` (`chrono.workspace = true`), `crates/vb_runtime/src/error/mod.rs:7` (`#[derive(thiserror::Error)]` is NOT used — RuntimeError uses `Debug, Clone` only for `Box`/`Arc` propagation).
- **Justification**: Workspace-pinned versions ensure reproducibility. `RuntimeError` does not derive `thiserror::Error`, so the new variant is a plain enum variant with two `u64`/`RunId` fields; no proc-macro dependence.
- **Scope**: Used by PO-001, PO-003, PO-006, PO-007 (variants depend on `#[non_exhaustive]` + manual `Debug`/`Display` impls; the existing `Display` impl at `display.rs` is updated to handle the new variant).
- **Risk**: the `Display` impl at `display.rs` is updated to add a `match` arm for `ResumeTimestampOverflow`. If the update is missed, `Display` returns a generic fallback. The source-lint gate `check-error-exhaustiveness.sh` enforces that the impl covers all variants.

## Cross-Reference Table

| Trusted base | Used by |
|---|---|
| TB-001 chrono | PO-001, PO-003, PO-004 |
| TB-002 i64::try_from | PO-001, PO-003, PO-007 |
| TB-003 non_exhaustive | PO-001, PO-003, PO-006, PO-007 |
| TB-004 clone-count atomic | PO-002, PO-004 |
| TB-005 recovery-readonly | PO-005 |
| TB-006 source-length 300 | PO-006 |
| TB-007 verus-binding gate | PO-006 |
| TB-008 mirror-drift gate | PO-006 |
| TB-009 loom-shard-singlethread | PO-005 |
| TB-010 single clone site | PO-001, PO-004, PO-006 |
| TB-011 forbid unsafe | PO-006 |
| TB-012 workspace pins | PO-001, PO-003, PO-006, PO-007 |

## Reduction Justification

- **Reduction 1 (16-variant enumeration)**: The mapper's input space is the closed 16-variant enum `RuntimeJournalEvent`. Proptest's `Proptest::arbitrary()` over the enum plus the explicit insertion of every variant in `chunk_004.rs:1077-1090` covers 100% of the input space. No symmetry reduction is needed because the variants are all small and the dispatch is a pure match.
- **Reduction 2 (u64 sentinel sweep)**: The conversion's input space is `u64`. The proptest sweeps 65536 random values plus six explicit sentinels `[0, 1, 1_700_000_000, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX]`. The reduction to "sentinels + random" is sound because the mapper is monotone in `timestamp_u64`: behavior depends only on whether `timestamp_u64 > i64::MAX`. The boundary at `i64::MAX` is captured by the explicit `i64::MAX as u64 + 1` sentinel.
- **Reduction 3 (Verus WEAK_EXTERN)**: Verus does not need to verify the full Rust code path. The mirror file `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` already encodes the production shape of `JournalEvent::RunResumed`; the new spec at `verification/verus/vb_cib14_resume_storage_map.rs` declares `assume_specification_targets = ["production::boundary_storage_event::Resumed_arm", "production::convert_resume_timestamp"]`. The drift gate (TB-007) fails CI if the production shape drifts.
- **Reduction 4 (loom 2-thread, 4 preemptions, 20000 branches)**: The temporal-replay seam (`storage_event` → `incident.rs` → `hydrate.rs`) has a single thread of execution in production. Loom with 2 threads (one writer, one replayer) and 4 preemptions is sufficient to expose any race between the conversion completion and the recovery read. The reduction is documented in the trusted-base entry TB-009.

## Non-Behavior Waivers

No waivers requested. All proof obligations address genuine behavior requirements; no behavior-affecting obligation is waived.

## Reduction Soundness

The four reductions above are sound for the corresponding claims. The proptest enumerations cover the typed-event dispatch table (Reduction 1) and the conversion boundary (Reduction 2) exhaustively in the ranges that matter. The Verus mirror (Reduction 3) is drift-gated so the production-binding remains accurate. The loom reduction (Reduction 4) is bounded by the single-shard, single-concurrent-task architectural constraint (TB-009). If any of these reductions becomes unsound (e.g., a new `RuntimeJournalEvent` variant is added without updating PO-004 or PO-007), the test failures from those proptest obligations will surface immediately.
