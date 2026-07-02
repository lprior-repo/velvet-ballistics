# Proof Coverage Matrix — vb-cib14

Maps every contract clause in `contract.md` to proof obligations and verifier lanes.

Status legend: `existing` — coverage already shipped in the codebase; `partial` — coverage exists but the obligation adds new evidence; `new` — coverage planned by this bead; `not_applicable` — risk surface provably absent; `superseded` — covered by another lane with evidence.

## C1 — Resumed Maps to RunResumed

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C1#mapper` | `storage_event(Resumed { run, timestamp }, seq)` returns `Ok(RunResumed { run, seq, timestamp: convert_resume_timestamp(timestamp, run)? })` | PO-001 (Verus mirror spec fn over `MirrorJournalEvent::RunResumed { run, seq, timestamp }`, `RunResumed { run: u64, seq, timestamp: u64 }` mirror at `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`, near the existing line range 616-624 + `run_id` 715 + `seq` 748 + `is_valid` 792/839) | verus (WEAK_EXTERN) | new |
| `C1#pass-through-fields` | `mapped.seq() == seq`, `mapped.run_id() == event.run_id()` | PO-002 (proptest over `(u64, RunId, EventSeq)` triple, PROPTEST_CASES=65536) | proptest | new |
| `C1#mappersite` | New `Resumed` arm lives in `boundary_storage_event`, mirroring `WaitScheduled → WaitScheduledEvent` | PO-004 (cargo-test extends the existing `storage_event_clones_the_event_exactly_once_per_dispatch` regression with a `Resumed` sample); PO-006 (source-lint forbids fallback arms) | cargo-test + source-lint | new |
| `C1#no-silent-rewrite` | Mapper never rewrites `Resumed` to `RunFailedEvent` | PO-006 (source-lint `check-panic-surface.sh` + `check-hot-cold-forbidden-apis.sh`); PO-007 (proptest 16-variant enumeration: every variant reaches the typed mapping or explicit `Ok(None)` no-op, never `RunFailedEvent` for `Resumed`) | source-lint + proptest | new |

## C2 — Timestamp Conversion Is Total And Explicit

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C2#totality-overflow` | `i64::try_from(timestamp)` failure ⇒ `Err(ResumeTimestampOverflow)` | PO-003 (proptest sweeps `u64` with sentinels `[0, 1, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX, 1_700_000_000]`) | proptest | new |
| `C2#totality-from_timestamp` | `from_timestamp` returning `None` ⇒ `Err(ResumeTimestampOverflow)` | PO-003 (proptest: realistic UNIX timestamps always produce `Some(_)`; far-future extreme i64 values produce `None`) | proptest | new |
| `C2#no-as-cast` | No `as i64`, no `unwrap`, no `expect`, no clamp, no wrap | PO-006 (source-lint `check-panic-surface.sh` + `check-hot-cold-forbidden-apis.sh` forbid these patterns) | source-lint | new |
| `C2#spec-binding` | Verus spec proves totality over the `u64` range | PO-001 (Verus mirror spec for the Resumed arm; assumes `production::boundary_storage_event::Resumed_arm` and `production::convert_resume_timestamp`) | verus (WEAK_EXTERN) | new |

## C3 — Storage Dispatch Totality (Paired With vb-edvbj)

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C3#compile-time-exhaustiveness` | `storage_event` is exhaustive over `RuntimeJournalEvent` | PO-004 (cargo-test extends the 16-variant enumeration test at `crates/vb_runtime/src/journal/tests/chunk_004.rs:1077-1090` to cover the `storage_event` mapper for every variant) | cargo-test | new |
| `C3#no-fallthrough` | No variant reaches the synthetic `RunFailedEvent` catch-all | PO-006 (source-lint: `clippy` + `rustc` compile-time exhaustiveness on `boundary_storage_event` `Resumed { .. }` arm becomes exhaustive after vb-edvbj's catch-all deletion); PO-007 (proptest enumeration) | source-lint + proptest | new |

## C4 — Single-Clone Invariant Preserved

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C4#clone-once-per-dispatch` | `STORAGE_EVENT_CLONE_COUNT` increases by exactly 1 per `storage_event` call | PO-004 (cargo-test extends `storage_event_clones_the_event_exactly_once_per_dispatch` at `crates/vb_runtime/src/journal/tests/chunk_002.rs:410-493` with a `Resumed` arm sample asserting `STORAGE_EVENT_CLONE_COUNT == 1`) | cargo-test | partial (existing test, new arm) |
| `C4#no-second-clone-in-arm` | Resumed arm does not introduce a second `clone_for_dispatch` | PO-006 (source-lint + code-review checklist: `boundary_storage_event` Resumed arm destructures once and passes parts forward) | source-lint | new |
| `C4#clone-pattern-preserved` | `&event` match pattern preserved | PO-006 (source-lint: `#[deny(clippy::needless_clone)]` rejects extra clones) | source-lint | new |

## C5 — Recovery/Replay Classifies RunResumed As Active

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C5#lifecycle-state-active` | `incident.rs::lifecycle_state(RunResumed) == LifecycleState::Active` | PO-005 (loom + proptest replay scenario: append `RunResumed` via `storage_event` → journal contains one `RunResumed` → `lifecycle_state` returns `Active`) | loom+proptest | new |
| `C5#hydrate-not-completed` | `recovery/hydrate.rs::is_in_flight_or_completed(RunResumed) == Ok(false)` | PO-005 (loom + proptest: same replay scenario asserts hydrate returns `Ok(false)` for `RunResumed` and does NOT return `Ok(true)` for the legacy-buggy `RunFailedEvent`) | loom+proptest | new |
| `C5#regression-buggy-shape` | Replay a journal where the bug rewrote `Resumed` as `RunFailedEvent` → hydration sees a `Failed` run | PO-005 (loom + proptest: explicit regression test that constructs a synthetic `RunFailedEvent` from a `Resumed` source run and asserts `lifecycle_state == LifecycleState::Failed`) | loom+proptest | new |
| `C5#refinement-obligation` | RRO-TLA-RESUME-001 (verification/tla/rust-refinement-obligations.jsonl:6) remains materializable | PO-005 (loom: source-ref expansion to `journal/chunk_002.rs:270-303` mapper arm + `shard/lifecycle/chunk_001.rs:291-367` FSM) | loom+proptest | partial (existing obligation, expanded source refs) |

## C6 — Seq And RunId Pass-Through

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C6#seq-passthrough` | `mapped_event.seq() == seq` | PO-002 (proptest: pass-through invariant for all 16 variants and random `(u64, RunId, EventSeq)` triples) | proptest | new |
| `C6#run-passthrough` | `mapped_event.run_id() == event.run_id()` | PO-002 (proptest: same input sweep) | proptest | new |
| `C6#no-derivation` | Mapper never derives `seq` from `timestamp` or zeros `run` | PO-001 (Verus spec: `requires(seq == input.seq)`, `ensures(mapped.seq == input.seq)`); PO-002 (proptest: confirm `seq` and `run` are sourced only from the `seq` parameter and `event.run_id()` respectively) | verus + proptest | new |

## C7 — Public Error Surface Adds ResumeTimestampOverflow

| Contract clause | Required behavior | Proof obligation | Verifier lane | Status |
|---|---|---|---|---|
| `C7#variant-shape` | `RuntimeError::ResumeTimestampOverflow { run: RunId, timestamp: u64 }` is a struct variant (not unit) | PO-007 (proptest: construct the variant via `Err(RuntimeError::ResumeTimestampOverflow { run, timestamp })`, assert `match` arms match the struct field count and types) | proptest | new |
| `C7#non-exhaustive` | `RuntimeError` remains `#[non_exhaustive]` | PO-006 (source-lint: `#[non_exhaustive]` attribute presence verified at `crates/vb_runtime/src/error/mod.rs:8`; adding a variant must not break any non-`#[non_exhaustive]` exhaustive match site in the workspace) | source-lint | new |
| `C7#diagnostic-carry` | Variant preserves the original `u64` timestamp | PO-003 (proptest: after overflow, `err` carries the original `timestamp: u64` value, not a clamped value) | proptest | new |

## Verifier Lane Summary

| Lane | Required obligations | Status |
|---|---|---|
| verus (mirror) | PO-001 | new |
| proptest (rust-local) | PO-002, PO-003, PO-007 | new |
| loom + proptest (temporal-replay) | PO-005 | new |
| source-lint | PO-006 (multi-script gate) | new |
| cargo-test | PO-004 (extends existing 16-variant + single-clone tests) | new |
| flux-rs | — | not_applicable (surface_absent) |
| miri | — | not_applicable (surface_absent) |
| cargo-fuzz | — | not_applicable (surface_absent) |
| kani | — | not_applicable (superseded_by_other_lane_with_evidence) |
| tla-plus | — | not_applicable (risk_out_of_scope, removed per master) |

## Bridge To Production Sources

| Contract clause | Production source ref | Verus mirror ref |
|---|---|---|
| C1, C2, C6 | `crates/vb_runtime/src/journal/chunk_002.rs:193-268` (modified `boundary_storage_event` arm + new helper) + `crates/vb_runtime/src/journal/chunk_002.rs:270-303` (top-level dispatcher dispatch site) | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` `MirrorJournalEvent::RunResumed` near line range 616-624 plus `run_id`/`seq`/`is_valid` mirror sites |
| C3, C4 | `crates/vb_runtime/src/journal/chunk_002.rs:270-324` (dispatcher + `clone_for_dispatch` helper at 318-324) | n/a |
| C5 | `crates/vb_storage/src/journal/incident.rs:203`, `crates/vb_storage/src/recovery/hydrate.rs:754`, `crates/vb_storage/src/recovery/replay/observation/normalize.rs:60,126`, `crates/vb_storage/src/recovery/replay/summary/apply.rs:79-81` | n/a |
| C7 | `crates/vb_runtime/src/error/mod.rs` (new variant + `#[non_exhaustive]` already present at line 8) | n/a |

## Lane Obligation Coverage Audit

| Lane | Required obligation IDs | Count | Pass |
|---|---|---|---|
| verus | PO-001 | 1 | ✅ |
| proptest | PO-002, PO-003, PO-007 | 3 | ✅ |
| loom+proptest | PO-005 | 1 | ✅ |
| source-lint | PO-006 | 1 (covers multiple lint scripts) | ✅ |
| cargo-test | PO-004 | 1 | ✅ |
