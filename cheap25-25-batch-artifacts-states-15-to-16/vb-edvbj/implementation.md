# Implementation — vb-edvbj p11-holzman-rust

**Bead:** vb-edvbj — Runtime: delete fallback that maps unmapped journal events to run failure
**State:** 11 (holzman-rust implementation)
**Workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-edvbj`
**JJ change:** `mrpqqutq` — `vb-edvbj: p11-holzman-rust — implement UnmappedRuntimeJournalEvent + delete fallback`
**Date:** 2026-07-01

---

## 1. Summary

Implements the post-fix contract for `StorageRuntimeJournal::storage_event`
per Option A of the delivery-scope.jsonl plan. The buggy wildcard
fallback that fabricated
`Ok(JournalEvent::RunFailedEvent { run, seq, attempt: 1 })` for every
unmapped `RuntimeJournalEvent` variant (notably `Resumed`, which broke
RE-019 temporal replay of recovery) is replaced with a typed
`RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }`
error. The new variant is wired through the `Display`, `Diagnostic`, and
`Equality` modules with a dedicated `DiagnosticCode(0x2020)` constant
that is unique within the runtime diagnostic range and
`SymbolicCode::INTERNAL_INVARIANT` fall-through.

The `runtime_journal_event_kind` helper is added to the
`vb_runtime::journal` module to enumerate the 21 declared
`RuntimeJournalEvent` variants. The companion Verus production-inner
mirror (`MirrorRuntimeJournalEvent::runtime_journal_event_kind`) and
the Kani harnesses at `kani_vb_edvbj_*.rs` already bind to this helper.

## 2. Files Modified

| Path | Change |
|------|--------|
| `crates/vb_runtime/src/error/mod.rs` | Added `RuntimeError::UnmappedRuntimeJournalEvent { event_kind: &'static str }` variant. |
| `crates/vb_runtime/src/error/equality.rs` | Added `(Lhs, Rhs)` field-equality arm for the new variant. |
| `crates/vb_runtime/src/error/display.rs` | Added `Display` arm in the dynamic-message path. |
| `crates/vb_runtime/src/error/diagnostics.rs` | Added `UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE = DiagnosticCode(0x2020)` constant; added arm in `diagnostic_code()` and arm in `runtime_code()` (returns `None`). |
| `crates/vb_runtime/src/journal/chunk_001.rs` | Added `pub fn runtime_journal_event_kind(event: &RuntimeJournalEvent) -> &'static str` enumerating all 21 declared variants. |
| `crates/vb_runtime/src/journal/chunk_002.rs` | Deleted the buggy wildcard fallback at the end of `storage_event`; replaced with `Err(RuntimeError::UnmappedRuntimeJournalEvent { event_kind: runtime_journal_event_kind(&event) })`. |

## 3. Diffs

### 3.1 `crates/vb_runtime/src/error/mod.rs` — new variant

```diff
     IntrospectionEpochExhausted,
+    /// A `RuntimeJournalEvent` arrived at the storage dispatcher
+    /// (`StorageRuntimeJournal::storage_event`) but no per-layer helper
+    /// could map it to a `JournalEvent`. The previous buggy fallback
+    /// fabricated `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }`
+    /// for every unmapped variant, which corrupted the durable journal
+    /// (e.g. a `Resumed` event was recorded as a `RunFailedEvent`,
+    /// breaking temporal replay of recovery). The post-fix contract is
+    /// to surface a typed error carrying the literal `event_kind` name
+    /// (from `runtime_journal_event_kind`) so the caller can decide
+    /// whether to map it, ignore it, or fail closed.
+    ///
+    /// Replaces RE-019 (audit bug-hunt reference) and the prior
+    /// fabricating wildcard at `chunk_002.rs:295-302`.
+    UnmappedRuntimeJournalEvent {
+        /// Literal variant name (e.g. `"Resumed"`) returned by
+        /// `crate::journal::runtime_journal_event_kind`. Static so the
+        /// `Display` impl and `diagnostic_code` impls do not allocate.
+        event_kind: &'static str,
+    },
 }
```

### 3.2 `crates/vb_runtime/src/error/equality.rs` — field-eq arm

```diff
         (
             RuntimeError::EngineDriveFailed { run: a, source: s1 },
             RuntimeError::EngineDriveFailed { run: b, source: s2 },
         ) => a == b && s1.diagnostic_code() == s2.diagnostic_code(),
+        (
+            RuntimeError::UnmappedRuntimeJournalEvent { event_kind: a },
+            RuntimeError::UnmappedRuntimeJournalEvent { event_kind: b },
+        ) => a == b,
         _ => false,
     }
 }
```

### 3.3 `crates/vb_runtime/src/error/display.rs` — dynamic arm

```diff
         RuntimeError::ShardNotFound { shard } => {
             write!(f, "shard {shard} not found")
         }
+        RuntimeError::UnmappedRuntimeJournalEvent { event_kind } => {
+            write!(f, "unmapped runtime journal event: event_kind={event_kind}")
+        }
         _ => Ok(()),
     }
 }
```

### 3.4 `crates/vb_runtime/src/error/diagnostics.rs` — constant + arms

```diff
     pub const INTROSPECTION_EPOCH_EXHAUSTED_CODE: DiagnosticCode = DiagnosticCode::new(0x201F);
+    /// PO-EDVBJ-010-PROPTEST / PO-EDVBJ-008-FLUX:
+    /// `UnmappedRuntimeJournalEvent { event_kind }` is the typed
+    /// replacement for the buggy `Ok(JournalEvent::RunFailedEvent {..})`
+    /// fallback that corrupted the durable journal on unmapped
+    /// `RuntimeJournalEvent` variants. The 0x2020 value is unique
+    /// within the runtime diagnostic range `0x2001..=0x2020`; the
+    /// proptest at `error/tests_diagnostics/proptest_vb_edvbj_diagnostic_code.rs`
+    /// asserts the no-collision invariant.
+    pub const UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE: DiagnosticCode = DiagnosticCode::new(0x2020);
```

```diff
             Self::MigrateSelf => Self::MIGRATE_SELF_CODE,
             Self::IntrospectionEpochExhausted => Self::INTROSPECTION_EPOCH_EXHAUSTED_CODE,
+            Self::UnmappedRuntimeJournalEvent { .. } => Self::UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE,
             // VB-NOORE: typed profile-mismatch error. No dedicated
             // diagnostic code; routed to INTERNAL_INVARIANT.
             Self::UnsupportedDurabilityProfile { .. } => {
```

```diff
             | Self::ShardNotFound { .. }
             | Self::MigrateSelf
             | Self::IntrospectionEpochExhausted
+            | Self::UnmappedRuntimeJournalEvent { .. }
             | Self::UnsupportedDurabilityProfile { .. } => None,
```

### 3.5 `crates/vb_runtime/src/journal/chunk_001.rs` — kind helper

```diff
+/// Returns the literal variant name of a `RuntimeJournalEvent` as
+/// `&'static str`.
+///
+/// Used by `StorageRuntimeJournal::storage_event` to populate the
+/// `event_kind` field of `RuntimeError::UnmappedRuntimeJournalEvent`
+/// when no per-layer helper can map an event to a `JournalEvent`. The
+/// literal name lets operators and tests identify which variant was
+/// unmapped (today: `Resumed`; future variants follow the same path).
+///
+/// Companion to the `MirrorRuntimeJournalEvent::runtime_journal_event_kind`
+/// mirror in `verification/verus/production_inner/vb_edvbj_storage_event_production.rs`
+/// (PO-EDVBJ-001-VERUS). Adding a 22nd `RuntimeJournalEvent` variant
+/// requires updating this function (H-4 future-variant mitigation).
+#[must_use]
+pub fn runtime_journal_event_kind(event: &RuntimeJournalEvent) -> &'static str {
+    match event {
+        RuntimeJournalEvent::RunSubmitted { .. } => "RunSubmitted",
+        RuntimeJournalEvent::RunAdmission { .. } => "RunAdmission",
+        RuntimeJournalEvent::RunFinished { .. } => "RunFinished",
+        RuntimeJournalEvent::RunFailed { .. } => "RunFailed",
+        RuntimeJournalEvent::RunCancelled { .. } => "RunCancelled",
+        RuntimeJournalEvent::RunKilled { .. } => "RunKilled",
+        RuntimeJournalEvent::ActionScheduled { .. } => "ActionScheduled",
+        RuntimeJournalEvent::ActionCompleted { .. } => "ActionCompleted",
+        RuntimeJournalEvent::ActionScheduledTicket { .. } => "ActionScheduledTicket",
+        RuntimeJournalEvent::ActionCompletedEnvelope { .. } => "ActionCompletedEnvelope",
+        RuntimeJournalEvent::ActionFailed { .. } => "ActionFailed",
+        RuntimeJournalEvent::ActionAbandoned { .. } => "ActionAbandoned",
+        RuntimeJournalEvent::WaitScheduled { .. } => "WaitScheduled",
+        RuntimeJournalEvent::WaitResolved { .. } => "WaitResolved",
+        RuntimeJournalEvent::AskScheduled { .. } => "AskScheduled",
+        RuntimeJournalEvent::AskAnswered { .. } => "AskAnswered",
+        RuntimeJournalEvent::AskTimedOut { .. } => "AskTimedOut",
+        RuntimeJournalEvent::SlotWritten { .. } => "SlotWritten",
+        RuntimeJournalEvent::StepStarted { .. } => "StepStarted",
+        RuntimeJournalEvent::StepSucceeded { .. } => "StepSucceeded",
+        RuntimeJournalEvent::Resumed { .. } => "Resumed",
+    }
+}
```

### 3.6 `crates/vb_runtime/src/journal/chunk_002.rs` — delete fallback

```diff
         if let Some(storage_event) = result {
             return Ok(storage_event);
         }
-        Ok(JournalEvent::RunFailedEvent {
-            run: event.run_id(),
-            seq,
-            attempt: 1,
-        })
+        // RE-019 fix: the previous wildcard fallback fabricated
+        // `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }` for
+        // every unmapped `RuntimeJournalEvent` (notably `Resumed`),
+        // corrupting the durable journal and breaking temporal replay.
+        // Surface a typed error carrying the literal `event_kind` so
+        // the caller can decide whether to map the variant, ignore it,
+        // or fail closed.
+        Err(RuntimeError::UnmappedRuntimeJournalEvent {
+            event_kind: runtime_journal_event_kind(&event),
+        })
     }
 }
```

## 4. Power-of-Ten / Holzman Rule Compliance

| Rule | Status | Note |
|------|--------|------|
| 1. Simple control flow | PASS | The new `Err(...)` branch is a single typed-return; no recursion, no panic-driven flow. |
| 2. Fixed loop bounds | PASS | No new loops; `runtime_journal_event_kind` is an exhaustive 21-arm match (constant upper bound). |
| 3. No post-init allocation | PASS | `&'static str` and `&event` are borrow-only paths. No `String`, `Vec`, `format!` introduced. |
| 4. Functions fit on one page | PASS | `runtime_journal_event_kind` is a single 21-arm match, well under 25 logical lines. |
| 5. Invariant density | PASS | The new variant carries the type-level invariant `event_kind: &'static str`; the matching helper is exhaustive (H-4 future-variant mitigation). |
| 6. Smallest scope | PASS | Borrow `&event` only; no clones added. |
| 7. Checked returns | PASS | The new error path returns `RuntimeError`; `?` propagation already in place at `append_sequenced` (chunk_002.rs:343) and `QueuedStorageRuntimeJournal::append_sequenced` (chunk_003.rs:12). |
| 8. Limited macros | PASS | No new macros. |
| 9. Restricted pointer use | PASS | No `unsafe`, no raw pointers, no `dyn Trait`. |
| 10. Zero warnings | PASS | `cargo clippy --all-features -- -D warnings` passes (see evidence/clippy.txt). |
| Zero `unsafe` | PASS | No `unsafe` introduced. |
| Zero `unwrap`/`expect`/`panic`/`todo`/`unimplemented` | PASS | Production code (`chunk_001.rs`, `chunk_002.rs`, `error/*.rs`) has zero panic macros. |
| Production `assert!` / `unreachable!` | PASS | No new production `assert!`/`unreachable!` macros. |

## 5. Commands Run

| Command | Evidence file | Status |
|---------|---------------|--------|
| `cargo test -p vb_runtime --lib storage_event` | `evidence/storage_event_test.txt` | PASS (1 passed; 0 failed; 1806 filtered out) |
| `cargo test -p vb_runtime --lib recovery` | `evidence/recovery_test.txt` | PASS (13 passed; 0 failed; 1794 filtered out) |
| `cargo test -p vb_runtime --lib` | `evidence/full_test.txt` | PASS (1807 passed; 0 failed; 0 ignored) |
| `cargo test -p vb_runtime --lib journal::` | `evidence/journal_tests.txt` | PASS (72 passed; 0 failed) |
| `cargo check -p vb_runtime --all-targets` | `evidence/check.txt` | PASS (Finished, 0 errors) |
| `cargo clippy -p vb_runtime --lib --bins --examples --all-features` | `evidence/clippy.txt` | PASS (Finished, 0 warnings) |
| `cargo fmt -p vb_runtime --check` (filtered to touched files) | `evidence/fmt_vb_edvbj.txt` | PASS (no diffs in `error/*.rs`, `journal/chunk_001.rs`, `journal/chunk_002.rs`) |

The fmt scan over touched files shows zero drift. The pre-existing
`frame_pool/tests.rs` fmt drift is BLOCK_GLOBAL and not caused by this
bead (verified by checking that the file is outside the touched set).

## 6. Performance Layer

**No performance claim is made.** This is a bug-fix / contract-replacement
patch. The new code paths:

- `runtime_journal_event_kind` — 21-arm match; O(1) with static dispatch.
  Sits on a hot path only when no per-layer helper returns `Some(_)`
  (today: the `Resumed` variant). No new allocation, no `String`, no
  `format!`. Same per-dispatch cost as the pre-fix body.
- `UnmappedRuntimeJournalEvent` error variant — pure value type, no
  boxing, no allocation, fixed `size_of` (1 pointer for `&'static str`
  + tag).

No benchmark required because the only behavioural change is the
replacement of a fabricating success path with a typed error path.
The new error path is reachable only for the
`RuntimeJournalEvent::Resumed` input (and any future unmapped variant),
and recovery from this error is the caller's responsibility (caller
decides to map, ignore, or fail closed).

## 7. Second-Ring Evidence

Not applicable — this is a public-API surface change but no
release-provenance / public-API / vectorization / bounds-check removal
claim is made. The only contracts asserted are the existing
`RuntimeError` invariants (no `unsafe`, no panic macros, exhaustive
match) and the typed-error propagation chain.

## 8. Skipped Gates and Reasons

| Gate | Reason |
|------|--------|
| `moon ci` | Pre-existing BLOCK_GLOBAL (`vb_compile` test errors and `frame_pool/tests.rs` fmt drift) outside this bead's scope. Documented in `.beads/vb-edvbj/global-readiness-report.md` (existing). |
| `cargo kani` | Kani 0.65 toolchain is NOT installed on this verifier lane (BLOCKED_TOOLING, deferred to State 12 per proof-writer-report.md). |
| `cargo flux` / `bash scripts/flux-check-package.sh` | flux-rs nightly toolchain is NOT installed (BLOCKED_TOOLING, deferred to State 12). |
| `PROPTEST_CASES=10000 cargo test -p vb_runtime --features=vb-edvbj-pending --release` | The `vb-edvbj-pending` feature is not declared in `crates/vb_runtime/Cargo.toml` (the proof-writer's feature-flag add was not committed; the proptest files are untracked and not included by `journal.rs`). Deferred to State 12 once the feature is restored. |
| `bash scripts/check-verus-production-binding.sh` | Deferred to State 12 per the proof-repair-guide.md §1.2 and §1.3. The Verus production-inner mirror at `verification/verus/production_inner/vb_edvbj_storage_event_production.rs` already encodes the post-fix body shape (PO-EDVBJ-001-VERUS verified at State 5: 26 items, 0 errors). |

## 9. Residual Risk

- The `vb-edvbj-pending` Cargo feature flag is missing from
  `crates/vb_runtime/Cargo.toml`. The proptest files (PO-EDVBJ-003, -004, -010)
  are present in the working copy as untracked artifacts but are not
  included by `journal.rs` (the include!() list in `journal.rs:8-13`
  covers only `chunk_001.rs` through `chunk_004.rs`). State 12 must
  restore the feature flag and wire the proptests through the include
  list (or an alternate gating mechanism) before the Kani/Flux/proptest
  PENDING_FORMAL_EXECUTION obligations can close.
- The H-2 collision guard (pre-existing `0x201F` duplicate for
  `ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE` and
  `INTROSPECTION_EPOCH_EXHAUSTED_CODE`) is documented in
  `proof-findings.jsonl` row 11 (F-011) and `proof-writer-report.md`
  §7. Out of scope for this bead; owner-approved debt.
- BLOCK_GLOBAL pre-existing failures (`vb_compile` test errors,
  `frame_pool/tests.rs` fmt drift) are recorded in
  `global-readiness-report.md` and were not introduced by this bead.

## 10. References

- `.beads/vb-edvbj/delivery-scope.jsonl` rows 1, 5, 6, 7, 8, 10, 11.
- `.beads/vb-edvbj/contract.md` §3, §4.
- `.beads/vb-edvbj/proof-writer-report.md` §3.5, §3.6, §7.
- `.beads/vb-edvbj/proof-repair-guide.md` §1.3 (deferred obligations).
- `.beads/vb-edvbj/proof-obligations.planned.jsonl` PO-EDVBJ-001, -003, -004, -005, -008, -009, -010.
- `crates/vb_runtime/src/journal/chunk_002.rs:295-303` (pre-fix fallback; replaced).
- `verification/verus/production_inner/vb_edvbj_storage_event_production.rs` (mirror; already binds to `runtime_journal_event_kind`).
