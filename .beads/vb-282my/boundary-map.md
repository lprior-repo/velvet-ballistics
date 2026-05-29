# Boundary Map — vb-282my

**Bead:** vb-282my (P1)
**Title:** TLA/Rust production boundaries for refinement harness placement
**Date:** 2026-05-29

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        TLA+ SPECTRA                               │
│  (temporal-design domain, not Rust implementation domain)         │
│                                                                    │
│  specs/                          verification/tla/                │
│  ├── AskAnswerLifecycle.tla      ├── ChooseSlotLowering.tla       │
│  ├── RetryFSM.tla               └── ChooseSlotReplay.tla          │
│  ├── RetryJournal.tla                                             │
│  ├── ResumeStateMachine.tla                                       │
│  └── admission_header_before_ack.tla                              │
│                                                                    │
│  ───────────────── REFINEMENT BRIDGE ─────────────────            │
│  (each RRO row is a bridge span: TLA model ↔ Rust source)         │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                    PURE CORE (vb_core)                             │
│                                                                    │
│  crates/vb_core/src/replay/choose.rs                              │
│  └── replay_choose_slot()  — pure function: no I/O, no time       │
│                                                                    │
│  RRO: CHOOSE-REPLAY-001                                           │
│  Harness placement: kani proof in vb_core crate                   │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│               COMPILE-TIME CORE (vb_compile)                       │
│                                                                    │
│  crates/vb_compile/src/mod_compile_lowering/part_02.rs            │
│  └── lower_canonical_choose()  — pure function, no I/O            │
│  crates/vb_compile/src/compile/mod.rs                             │
│  ├── lower_choose()          — builds CompiledNodeKind            │
│  └── validate_branch_route() — post-compile validation            │
│                                                                    │
│  RRO: CHOOSE-LOWERING-001                                         │
│  Harness placement: kani proof in vb_compile crate                │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│              IMPERATIVE SHELL (vb_runtime)                         │
│                                                                    │
│  crates/vb_runtime/src/shard/                                     │
│                                                                    │
│  ┌─ lifecycle/chunk_001.rs ─┐  ┌─ lifecycle/chunk_002.rs ─┐     │
│  │ handle_submit()           │  │ handle_ask_answer()       │     │
│  │  ├─ admission header      │  │  └─ pending timer check   │     │
│  │  │  appends BEFORE state  │  │  └─ SlotWritten journal   │     │
│  │  │  insert                │  │  └─ AskAnswered journal   │     │
│  │  ├─ handle_resume()       │  │  └─ drive_run()           │     │
│  │  │  └─ append Resumed     │  └───────────────────────────┘     │
│  │  │     BEFORE drive       │                                     │
│  │  └─ restore_resumable_    │                                     │
│  │     after_drive_failure() │  ┌─ transitions.rs ─────────┐     │
│  └───────────────────────────┘  │ apply()                   │     │
│                                  │  └─ RuntimeState FSM     │     │
│  ┌─ helpers.rs ─────────────┐  │ await_timer()             │     │
│  │ record_retry_attempt()   │  │  └─ AskScheduled journal  │     │
│  │  └─ monotonicity check   │  │     BEFORE pending_timers │     │
│  │  └─ overflow fail-closed │  └───────────────────────────┘     │
│  └──────────────────────────┘                                     │
│                                                                    │
│  RROs: ASK-ANSWER-001, RETRY-FSM-001, RESUME-001, ADMISSION-001  │
│  Harness placement: kani proof in vb_runtime crate                │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│              STORAGE BOUNDARY (vb_storage)                         │
│                                                                    │
│  crates/vb_storage/src/journal/internal.rs                        │
│  ├── append_unpersisted()          — strict duplicate rejection    │
│  └── append_queued_unpersisted()   — idempotent duplicate path    │
│  crates/vb_storage/src/keys.rs                                     │
│  └── run_event_key()               — (run, seq) key encoding      │
│                                                                    │
│  RRO: RETRY-JOURNAL-001                                            │
│  Harness placement: kani proof in vb_storage crate                │
└──────────────────────────────────────────────────────────────────┘
```

## Boundary Taxonomy

### Boundary Type 1: Pure Core

Code that is deterministic, I/O-free, time-free, storage-free. Can be verified with Kani or Verus directly.

| File | Symbol | RRO | Core Property |
|------|--------|-----|---------------|
| `crates/vb_core/src/replay/choose.rs` | `replay_choose_slot` | CHOOSE-REPLAY-001 | Deterministic branch iteration with checked indexing |
| `crates/vb_compile/src/mod_compile_lowering/part_02.rs` | `lower_canonical_choose` | CHOOSE-LOWERING-001 | Compile-time validation with bounded iteration |
| `crates/vb_compile/src/compile/mod.rs` | `lower_choose` | CHOOSE-LOWERING-001 | Node construction |
| `crates/vb_compile/src/compile/mod.rs` | `validate_branch_route` | CHOOSE-LOWERING-001 | Post-compile validation |

**Harness approach:** Kani `#[kani::proof]` with `kani::any()`. No stubs needed for time/I/O.

### Boundary Type 2: Imperative Shell (Stateful Core)

Code that modifies internal state (maps, vectors, counters) but does not call external I/O or storage directly. The state mutations are the property of interest.

| File | Symbol | RRO | Stateful Property |
|------|--------|-----|-------------------|
| `crates/vb_runtime/src/shard/helpers.rs` | `record_retry_attempt` | RETRY-FSM-001 | Monotonic attempt counter, overflow fail-closed |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `handle_submit` (admission portion) | ADMISSION-001 | Journal before RunState insert ordering |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `append_admission_header_journal_event` | ADMISSION-001 | Error mapping: append failure → AdmissionHeaderPersistenceFailed |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `handle_resume` | RESUME-001 | RuntimeState guard, append-then-drive ordering |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `append_resumed_event` | RESUME-001 | Append Resumed, rollback on journal failure |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `restore_resumable_after_drive_failure` | RESUME-001 | Drive failure → ResumeRollback (preserves journal) |
| `crates/vb_runtime/src/shard/transitions.rs` | `apply` | RESUME-001 | RuntimeState FSM transitions |
| `crates/vb_runtime/src/shard/transitions.rs` | `await_timer` | ASK-ANSWER-001 | AskScheduled journal before pending_timers insert |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `handle_ask_answer` | ASK-ANSWER-001 | Pending timer guard, SlotWritten before AskAnswered |

**Harness approach:** Kani `#[kani::proof]` with `kani::any()` for state setup. May need to stub journal `append_journal_event` to return synthetic success/failure.

### Boundary Type 3: Storage Boundary

Code that interfaces with the Fjall LSM-tree storage engine. Must verify the key-space semantics and idempotency properties.

| File | Symbol | RRO | Storage Property |
|------|--------|-----|-----------------|
| `crates/vb_storage/src/journal/internal.rs` | `append_unpersisted` | RETRY-JOURNAL-001 | Strict duplicate-key rejection |
| `crates/vb_storage/src/journal/internal.rs` | `append_queued_unpersisted` | RETRY-JOURNAL-001 | Idempotent duplicate: same event → Ok, different event → Err |
| `crates/vb_storage/src/keys.rs` | `run_event_key` | RETRY-JOURNAL-001 | Deterministic key encoding: `[0x11][run_u64_be][seq_u64_be]` |

**Harness approach:** Kani `#[kani::proof]` with in-memory keyspace (Fjall provides `Keyspace::open` with `Config::default()` for temp). Must verify `contains_key` + `insert` semantics for duplicate detection.

### Boundary Type 4: Error Mapping Boundary

Code that maps between error types. Does not contain behavior logic but must be verified for correctness of the mapping.

| File | Symbol | RRO | Error Mapping Property |
|------|--------|-----|----------------------|
| `crates/vb_runtime/src/error/conversions.rs` | `admission_header_persistence_failed` | ADMISSION-001 | Wraps StorageJournalAppend → AdmissionHeaderPersistenceFailed |

**Harness approach:** Simple Kani verification: check that `RuntimeError::admission_header_persistence_failed(StorageJournalAppend{...})` returns `AdmissionHeaderPersistenceFailed{...}`.

### Boundary Type 5: Time Boundary

Code that calls `Instant::now()` or similar. Not directly in the refinement claims, but used in timestamp generation.

| File | Symbol | Affected RRO | Time Property |
|------|--------|-------------|---------------|
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `current_timestamp()` | RESUME-001 | Timestamp monotonicity not part of TLA+ claim; safe to stub |

**Harness approach:** Stub `current_timestamp()` to return a `kani::any::<u64>()`. TLA+ model does not depend on real time ordering.

### Boundary Type 6: Async Shell

Code that bridges to tokio runtime. Not in any RRO scope. RRO rows cover only synchronous state transitions.

## Cross-Crate Dependency Graph

```
vb_compile ──▶ vb_core ──▶ vb_runtime ──▶ vb_storage
    │              │            │               │
    │              │            │               │
    ▼              ▼            ▼               ▼
CHOOSE-        CHOOSE-     ASK-ANSWER-001   RETRY-
LOWERING-001   REPLAY-001  RETRY-FSM-001    JOURNAL-001
                            RESUME-001
                            ADMISSION-001
```

**Implication:** A cross-crate harness cannot import from both `vb_compile` and `vb_core` within a single Kani proof (separate crate boundaries). Each RRO's harness must reside in the crate where the target symbol is defined.

### Exceptions

- **Admission shared state:** `handle_submit` in `vb_runtime` calls `append_journal_event` which lives in `vb_runtime/src/shard/journal.rs`. Same crate, not cross-crate.
- **Retry policy:** `record_retry_attempt` in `vb_runtime` uses `RetryPolicy` from `vb_core`. This is a cross-crate dependency but `RetryPolicy` is a simple struct — can be constructed inline in the Kani harness without stubbing `vb_core`.

## What Belongs Where

| Artifact | Location | Rationale |
|----------|----------|-----------|
| Kani harness for CHOOSE-LOWERING-001 | `crates/vb_compile/src/verification/kani/` | Same crate as `lower_canonical_choose` |
| Kani harness for CHOOSE-REPLAY-001 | `crates/vb_core/src/replay/kani/` or `crates/vb_core/src/verification/kani/` | Same crate as `replay_choose_slot` |
| Kani harness for ASK-ANSWER-001 | `crates/vb_runtime/src/verification/kani/` | Same crate as `handle_ask_answer` and `await_timer` |
| Kani harness extension for RETRY-FSM-001 | `crates/vb_runtime/src/verification/kani/kani_shard_lifecycle_harnesses.rs` | Existing file, extend with new proofs |
| Kani harness for RETRY-JOURNAL-001 | `crates/vb_storage/src/verification/kani/` | Same crate as `append_unpersisted` and `run_event_key` |
| Kani harness for RESUME-001 | `crates/vb_runtime/src/verification/kani/` | Same crate as `handle_resume` and `apply` |
| Kani harness for ADMISSION-001 | `crates/vb_runtime/src/verification/kani/` | Same crate as `handle_submit` and `append_admission_header_journal_event` |
| Flux refinements (alternative) | Inline `#[sig]` in production `.rs` files | Flux annotations live on the production functions |
| Verus specs (alternative) | `crates/*/src/verification/verus/` or inline | Verus specs can be separate files but must bind to `exec fn` |
| Proptest properties (alternative) | `crates/*/tests/` or inline `#[cfg(test)]` | Proptest tests run via `cargo test` |
| Waiver documents | `verification/waivers/` or inline in RRO review | Waivers are reviewer-scrutinized, not compiled |
| TLA+ model updates | `specs/` or `verification/tla/` | TLA+ stays separate; no Rust compilation dependency |

## Harness Isolation Rules

1. **No cross-crate harnesses for Kani.** A Kani proof in `vb_runtime` cannot `use vb_core::replay::choose::replay_choose_slot` for verification (only for generating helper values). Verification targets must be in the same crate.

2. **Journal stubbing is allowed.** `append_journal_event` calls Fjall storage. Kani harnesses in `vb_runtime` MUST stub `append_journal_event` to return synthetic `Ok(())` or `Err(JournalError::...)` to explore both paths.

3. **Time stubbing is allowed and encouraged.** `current_timestamp()` returns wall-clock time. Harnesses must stub it to `kani::any::<u64>()` or a fixed value.

4. **Capability gates can be bypassed in harness.** Input validation like `ActiveRunCapacityExceeded` guards can be satisfied by providing small input sets rather than being the target of verification (unless the guard itself is the claim).

5. **No storage engine in Kani.** Do not attempt to run the real Fjall LSM-tree under Kani. Use an in-memory mock or stub the storage interface.

## Unverifiable Boundaries

Some properties are outside the scope of Kani/Flux/Verus:

| Property | Why Unverifiable | Mitigation |
|----------|-----------------|------------|
| Weak fairness liveness (RetryFSM) | Kani verifies safety, not liveness. Weak fairness requires unbounded iteration. | TLA+ covers liveness under bounded fairness. Kani verifies safety: no retry after max, terminal typing. |
| Journal disk durability | Kani cannot verify disk write ordering. | Trust the `Fsync` path in `vb_storage`. Kani verifies key-space semantics only. |
| Cross-shard concurrency | Shards are async; Kani is single-threaded. | TLA+ models multi-shard with `RunIds`. Behavior tests cover concurrent scenarios. |
| Admission resource budgets | Budget computation is arithmetic-heavy with overflow guards. | Kani verifies error-mapping paths. Budget computation correctness is a separate RRO. |
