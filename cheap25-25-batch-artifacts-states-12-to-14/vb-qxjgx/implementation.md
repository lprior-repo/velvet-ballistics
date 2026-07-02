# Implementation — vb-qxjgx StepSucceeded / SlotWritten Split

- **bead_id**: vb-qxjgx
- **state**: 11 (holzman-rust)
- **workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
- **jj change id**: `ttulypyv`
- **jj commit id**: `96030faf`
- **parent commit**: `ywnswumt 1b72c500` (p5-proof-writer: Kani x5 + proptest x2 for vb-qxjgx)
- **controller**: femdation
- **subagent**: holzman-rust (direct child of femdation)
- **CURRENT_SCHEMA_VERSION**: `1` (UNCHANGED — back-compat is legacy envelope-12 tolerance, NOT a schema bump)
- **captured_at**: 2026-07-01T19:15:00Z

## Summary

Landed the production change that splits the `JournalEvent::StepSucceeded` and
`JournalEvent::SlotWrittenEvent` projections from the pre-fix OR-collapse
`Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten`
into two distinct match arms. Added a new `RecordKind::StepSucceeded = 33` arm,
extended the parity gate to honor a typed `LegacyEnvelopeBinding { Exact | Legacy { accepted_ids } }`
back-compat accessor (StepSucceeded admits envelope ids `{12, 33}`), and substituted
`RecordKind::SlotWritten` → `RecordKind::StepSucceeded` in the 13 step-closing positions
of the durability matrix.

All 7 production-bound proof obligations (PO-QXJGX-001..007) now have an in-tree
production surface to bind against. The 4 forward-looking E0599 errors that the
proof-writer flagged at State 5 (PENDING_FORMAL_EXECUTION) clear post-implementation.
The 6 back-compat unit tests at `codec/tests.rs:1617-1791` (added by the proof-writer)
PASS post-implementation. The 5 Kani harnesses + 2 proptest files compile cleanly
under their feature gates and would resolve their PENDING_FORMAL_EXECUTION state once
the pre-existing `vb_core/src/frame/parts/kani_helpers.rs` BLOCKED_TOOLING blocker
(TBR-001) is repaired by its owner.

## Production Code Changes (8 files)

### 1. `crates/vb_storage/src/records.rs` — Add `StepSucceeded = 33` variant

```diff
@@ crates/vb_storage/src/records.rs:186 @@
     ActionAbandoned = 32,
+    /// Step succeeded event.
+    ///
+    /// Distinct from `SlotWritten = 12` post the record-kind split:
+    /// `StepSucceeded` closes a step (the step's terminal event), while
+    /// `SlotWritten` records a raw slot write. The parity gate accepts
+    /// envelope ids `{12, 33}` for `StepSucceeded` (legacy envelope-12
+    /// tolerance) and only envelope id `12` for `SlotWrittenEvent`
+    /// payloads (POST-005, POST-007, INV-001).
+    StepSucceeded = 33,
     /// Run finished event.
     RunFinished = 22,
```

```diff
@@ crates/vb_storage/src/records.rs:247 @@
             Self::WaitResolved => 31,
             Self::ActionAbandoned => 32,
+            Self::StepSucceeded => 33,
             Self::Blob => 40,
```

### 2. `crates/vb_storage/src/events.rs:406` — Split the OR-pattern collapse

```diff
@@ crates/vb_storage/src/events.rs:406 @@
             Self::StepStarted { .. } => RecordKind::StepStarted,
-            Self::StepSucceeded { .. } | Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
+            Self::StepSucceeded { .. } => RecordKind::StepSucceeded,
+            Self::SlotWrittenEvent { .. } => RecordKind::SlotWritten,
```

### 3. `crates/vb_storage/src/codec/validation.rs` — Extend `is_known_record_kind` and `validate_kind_family`

```diff
@@ crates/vb_storage/src/codec/validation.rs:23 @@
 pub(crate) const fn is_known_record_kind(kind: u16) -> bool {
-    matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | 40 | 50)
+    matches!(kind, 1 | 2 | 3 | 10..=29 | 30 | 31 | 32 | 33 | 40 | 50)
 }
```

```diff
@@ crates/vb_storage/src/codec/validation.rs:46 @@
         MAGIC_JOURNAL_EVENT => {
             matches!(kind, 10..=29)
                 || kind == RecordKind::WaitResolved.id()
                 || kind == RecordKind::ActionAbandoned.id()
+                || kind == RecordKind::StepSucceeded.id()
         }
```

### 4. `crates/vb_storage/src/codec/kind_parity.rs` — Add `LegacyEnvelopeBinding` + use in parity impl

```diff
@@ crates/vb_storage/src/codec/kind_parity.rs:20 @@
 use crate::{
     JournalError,
+    events::JournalEvent,
     records::{BlobRecord, CompiledIrRecord, RunHeaderRecord, WorkflowSourceRecord},
     recovery::RunSnapshot,
     types::RecordEnvelope,
 };
+
+/// Per-event binding between the canonical `RecordKind` and the envelope
+/// kinds admitted by the parity gate.
+///
+/// `StepSucceeded` payloads admit envelope ids `{12, 33}` (POST-005):
+/// the canonical id `33` emitted by post-fix writers, and the legacy
+/// id `12` emitted by pre-fix writers (back-compat tolerance, no
+/// schema bump — see `CURRENT_SCHEMA_VERSION = 1` invariant). All
+/// other journal variants admit only their canonical id, captured by
+/// the [`LegacyEnvelopeBinding::Exact`] arm.
+///
+/// The parity gate consumes the binding via
+/// [`LegacyEnvelopeBinding::admits`] to honor both the writer-side
+/// canonical id and the read-side legacy tolerance in lockstep. This
+/// keeps the cross-bind rejection invariant (POST-007) intact while
+/// preserving back-compat with journals written before the
+/// `StepSucceeded` / `SlotWrittenEvent` record-kind split.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum LegacyEnvelopeBinding {
+    /// Accept only the canonical envelope id (the event's
+    /// `record_kind().id()`).
+    Exact,
+    /// Accept the canonical envelope id and any of `accepted_ids` for
+    /// back-compat with journals written before the record-kind split.
+    Legacy {
+        /// Envelope ids admitted in addition to the canonical id.
+        accepted_ids: &'static [u16],
+    },
+}
+
+impl LegacyEnvelopeBinding {
+    /// Returns the binding for a `JournalEvent`.
+    ///
+    /// The binding is variant-keyed: only `StepSucceeded` admits a
+    /// legacy id set (POST-005). Every other variant returns
+    /// [`LegacyEnvelopeBinding::Exact`].
+    #[must_use]
+    pub const fn for_journal_event(event: &JournalEvent) -> Self {
+        match event {
+            JournalEvent::StepSucceeded { .. } => Self::Legacy {
+                accepted_ids: &[12, 33],
+            },
+            _ => Self::Exact,
+        }
+    }
+
+    /// Returns `true` if `envelope_kind` is admitted by this binding.
+    ///
+    /// - `Exact`: `envelope_kind == canonical_kind`.
+    /// - `Legacy { accepted_ids }`: `envelope_kind == canonical_kind`
+    ///   or `envelope_kind ∈ accepted_ids`.
+    #[must_use]
+    pub fn admits(self, envelope_kind: u16, canonical_kind: u16) -> bool {
+        match self {
+            Self::Exact => envelope_kind == canonical_kind,
+            Self::Legacy { accepted_ids } => {
+                envelope_kind == canonical_kind || accepted_ids.contains(&envelope_kind)
+            }
+        }
+    }
+}
```

```diff
@@ crates/vb_storage/src/codec/kind_parity.rs:50 @@
 impl EnforceKindParity for crate::JournalEvent {
     fn enforce_kind_parity(envelope: &RecordEnvelope, value: &Self) -> Result<(), JournalError> {
         let payload_kind = value.record_kind().id();
-        if envelope.record_kind != payload_kind {
+        let binding = LegacyEnvelopeBinding::for_journal_event(value);
+        if !binding.admits(envelope.record_kind, payload_kind) {
             return Err(JournalError::RecordKindPayloadMismatch {
                 envelope_kind: envelope.record_kind,
                 payload_kind,
             });
         }
         if !value.is_valid() {
             return Err(JournalError::InvalidEvent);
         }
         Ok(())
     }
 }
```

### 5. `crates/vb_storage/src/codec/mod.rs` — Re-export `LegacyEnvelopeBinding` + use in `validate_journal_event_record_kind`

```diff
@@ crates/vb_storage/src/codec/mod.rs:20 @@
-pub use self::kind_parity::EnforceKindParity;
+pub use self::kind_parity::{EnforceKindParity, LegacyEnvelopeBinding};
```

```diff
@@ crates/vb_storage/src/codec/mod.rs:97 @@
-/// Validates that the envelope kind exactly matches the decoded journal payload variant.
+/// Validates that the envelope kind matches the decoded journal payload variant.
+///
+/// The check is variant-keyed: the [`LegacyEnvelopeBinding`] for the
+/// decoded event determines the admitted envelope id set. `StepSucceeded`
+/// payloads admit envelope ids `{12, 33}` (POST-005, back-compat with
+/// pre-fix journals). Every other variant admits only its canonical id
+/// (the variant's `RecordKind::id()`).
 pub fn validate_journal_event_record_kind(
     envelope: &RecordEnvelope,
     event: &JournalEvent,
 ) -> Result<(), JournalError> {
     let payload_kind = event.record_kind().id();
-    if envelope.record_kind == payload_kind {
+    let binding = LegacyEnvelopeBinding::for_journal_event(event);
+    if binding.admits(envelope.record_kind, payload_kind) {
         Ok(())
     } else {
         Err(JournalError::RecordKindPayloadMismatch {
             envelope_kind: envelope.record_kind,
             payload_kind,
         })
     }
 }
```

### 6. `crates/vb_runtime/src/durability_matrix.rs` — Substitute SlotWritten → StepSucceeded in 13 positions

The 13 substitutions (each row's `journal_events` slice in step-closing
positions, one per row for set/do/choose/for_each/parallel/repeat/wait and two
each for collect/aggregate/ask):

```diff
@@ crates/vb_runtime/src/durability_matrix.rs:75,89,100,110,120,132-133,146-147,158,171,186-187 @@
-    journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],  // set
+    journal_events: &[RecordKind::StepStarted, RecordKind::StepSucceeded],
-            RecordKind::SlotWritten,                                          // do
+            RecordKind::StepSucceeded,
-    journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],  // choose
+    journal_events: &[RecordKind::StepStarted, RecordKind::StepSucceeded],
-    journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],  // for_each
+    journal_events: &[RecordKind::StepStarted, RecordKind::StepSucceeded],
-    journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],  // parallel
+    journal_events: &[RecordKind::StepStarted, RecordKind::StepSucceeded],
-            RecordKind::SlotWritten,                                          // collect (1)
+            RecordKind::StepSucceeded,
-            RecordKind::SlotWritten,                                          // collect (2)
+            RecordKind::StepSucceeded,
-            RecordKind::SlotWritten,                                          // aggregate (1)
+            RecordKind::StepSucceeded,
-            RecordKind::SlotWritten,                                          // aggregate (2)
+            RecordKind::StepSucceeded,
-    journal_events: &[RecordKind::StepStarted, RecordKind::SlotWritten],  // repeat
+    journal_events: &[RecordKind::StepStarted, RecordKind::StepSucceeded],
-            RecordKind::SlotWritten,                                          // wait
+            RecordKind::StepSucceeded,
-            RecordKind::SlotWritten,                                          // ask (1)
+            RecordKind::StepSucceeded,
-            RecordKind::SlotWritten,                                          // ask (2)
+            RecordKind::StepSucceeded,
```

The `finish` row retains `RecordKind::RunFinished` (unchanged — `finish` is NOT a
step-closing primitive; per the proptest H1, only `set`, `do`, `choose`,
`for_each`, `parallel`, `collect`, `aggregate`, `repeat`, `wait`, `ask` are
step-closing).

### 7. `crates/vb_storage/src/codec/flux_validation.rs` — Add `kind == 33` to literal-sync sources

```diff
@@ crates/vb_storage/src/codec/flux_validation.rs:14 @@
 #[sig(fn(kind: u16) -> bool[{
     kind == 1 || kind == 2 || kind == 3 || kind == 30 || kind == 31 || kind == 32 ||
+    kind == 33 ||
     kind == 40 || kind == 50 ||
     (kind >= 10 && kind <= 29)
 }])]
```

```diff
@@ crates/vb_storage/src/codec/flux_validation.rs:33 @@
 #[sig(fn(kind: u16) -> bool[{
-    ((kind >= 10 && kind <= 29) || kind == 31 || kind == 32) ==
+    ((kind >= 10 && kind <= 29) || kind == 31 || kind == 32 || kind == 33) ==
         model_validate_kind_family_ok(kind)
 }])]
```

### 8. `crates/vb_cli/src/status.rs:337` — Fix the StepSucceeded display mapping

The CLI's `build_explain_entry` was a SECOND consumer of the OR-collapse:
`JournalEvent::StepSucceeded` was displayed as `RecordKind::SlotWritten`.
Updated to display `RecordKind::StepSucceeded`:

```diff
@@ crates/vb_cli/src/status.rs:337 @@
         JournalEvent::StepSucceeded { step: s, .. } => (
             "StepSucceeded",
             None,
-            Some(vb_storage::RecordKind::SlotWritten),
+            Some(vb_storage::RecordKind::StepSucceeded),
             Some(s.get()),
             None,
         ),
```

## Test Code Updates (5 files)

### A. `crates/vb_storage/src/codec/tests/replay_integrity.rs:224-232` — Update the journal-family admission test

The pre-fix test asserted `is_known_record_kind(33) == false` and
`validate_kind_family(MAGIC_JOURNAL_EVENT, 33).is_err() == true`. Both
assertions are inverted post-fix (33 is now `StepSucceeded`, admitted):

```diff
@@ crates/vb_storage/src/codec/tests/replay_integrity.rs:224 @@
-    // Kinds outside the journal range remain rejected.
-    assert!(!is_known_record_kind(33), "kind 33 must remain unknown");
-    assert!(
-        matches!(
-            validate_kind_family(MAGIC_JOURNAL_EVENT, 33),
-            Err(JournalError::RecordKindFamilyMismatch { .. })
-        ),
-        "kind 33 must be rejected for journal magic"
-    );
+    // StepSucceeded (33) is the new journal kind for the
+    // StepSucceeded / SlotWrittenEvent record-kind split (vb-qxjgx).
+    assert!(
+        is_known_record_kind(33),
+        "kind 33 (StepSucceeded) must be known"
+    );
+    assert!(
+        validate_kind_family(MAGIC_JOURNAL_EVENT, 33).is_ok(),
+        "kind 33 (StepSucceeded) must be admitted for journal magic"
+    );
 }
```

### B. `crates/vb_storage/src/tests.rs:3317-3326` — Update the all-variants record-kind projection test

```diff
@@ crates/vb_storage/src/tests.rs:3325 @@
-            RecordKind::SlotWritten
+            RecordKind::StepSucceeded
         );
```

### C. `crates/vb_runtime/src/durability_matrix/tests.rs:51` — Update the set-row test

```diff
@@ crates/vb_runtime/src/durability_matrix/tests.rs:51 @@
-    assert!(row.journal_events.contains(&RecordKind::SlotWritten));
+    assert!(row.journal_events.contains(&RecordKind::StepSucceeded));
```

### D. `crates/vb_storage/src/kani_record_kind.rs:177-188` — Delete `check_unknown_kind_rejected`

Per TBR-010 (proof-writer report, line 88-90 of `proof-to-rust-map.md`), the
pre-existing kani harness `check_unknown_kind_rejected` asserts
`validate_kind_family(MAGIC_JOURNAL_EVENT, 33).is_err() == true` (line 184-187).
Post-implementation, kind 33 is admitted as `StepSucceeded`; the harness
would FAIL. The replacement is `check_kind_33_journal_family_admit` at
`kani_record_kind_journal_family_33.rs:H2` (PO-QXJGX-003). Deleted in
lockstep with the production change.

```diff
@@ crates/vb_storage/src/kani_record_kind.rs:177-188 @@
-/// PO-KANI-004-H6: Unknown kind 33 must be rejected.
-/// Kind 32 is now ActionAbandoned.
-#[kani::proof]
-fn check_unknown_kind_rejected() {
-    let kind: u16 = 33;
-    let magic: u32 = crate::MAGIC_JOURNAL_EVENT;
-    let result = crate::codec::validation::validate_kind_family(magic, kind);
-    kani::assert(
-        result.is_err(),
-        "unknown kind 33 must be rejected by validate_kind_family",
-    );
-}
-
 /// PO-KANI-004-H6b: Kind 31 (WaitResolved) must now be admitted for MAGIC_JOURNAL_EVENT.
```

### E. `crates/vb_storage/src/lib.rs:98` — Update the PENDING_FORMAL_EXECUTION comment

```diff
@@ crates/vb_storage/src/lib.rs:97 @@
 // P1 bug: split StepSucceeded from SlotWrittenEvent, new RecordKind::StepSucceeded = 33,
 // legacy envelope-12 acceptance. Forward-looking: compiles only after
-// implementation lands (Implementation landed in State 11
-// holzman-rust). Status: PENDING_FORMAL_EXECUTION.
+// implementation lands (State 11 holzman-rust). Status: PENDING_FORMAL_EXECUTION.
```

(Comment was always confusingly worded; cleaned up the wording to reflect the
post-implementation state.)

## Pre-Existing Test Files Untouched (proof-writer authored)

- `crates/vb_storage/src/codec/tests.rs:1617-1791` — 6 back-compat unit tests
  (POST-001, POST-002, POST-005, POST-006, POST-007, INV-001)
- `crates/vb_storage/src/kani_record_kind_id_step_succeeded.rs` — PO-QXJGX-001 (3 harnesses)
- `crates/vb_storage/src/kani_record_kind_projection_split.rs` — PO-QXJGX-002 (3 harnesses)
- `crates/vb_storage/src/kani_record_kind_journal_family_33.rs` — PO-QXJGX-003 (6 harnesses)
- `crates/vb_storage/src/kani_record_kind_parity_legacy_envelope.rs` — PO-QXJGX-004 (7 harnesses)
- `crates/vb_storage/src/kani_record_kind_decode_round_trip.rs` — PO-QXJGX-005 (3 harnesses)
- `crates/vb_storage/tests/proptest_replay_summary_step_succeeded_split.rs` — PO-QXJGX-006 (4 properties)
- `crates/vb_runtime/tests/proptest_durability_matrix_step_succeeded.rs` — PO-QXJGX-007 (5 properties)

## Commands Run (Evidence)

All commands run from `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qxjgx`
(workdir root). Raw output captured under `.beads/vb-qxjgx/evidence/`.

### Targeted tests

```
$ cargo test -p vb_storage --tests
cargo test: 1678 passed (17 suites, 12.43s)
                                                (evidence/1782953000-cargo-test-vb_storage.txt)

$ cargo test -p vb_runtime --tests
cargo test: 2348 passed, 1 ignored (35 suites, 1.93s)
                                                (evidence/1782953001-cargo-test-vb_runtime.txt)
```

### 6 back-compat unit tests (POST-001..POST-007)

```
$ cargo test -p vb_storage --lib --no-fail-fast step_succeeded_event_maps_to_step_succeeded_kind -- --nocapture
cargo test: 1 passed, 1534 filtered out (1 suite, 0.01s)
                                                (evidence/1782953003a-bc-test-1.txt)

$ cargo test -p vb_storage --lib --no-fail-fast slot_written_event_maps_to_slot_written_kind_unchanged -- --nocapture
cargo test: 1 passed, 1534 filtered out (1 suite, 0.00s)
                                                (evidence/1782953003b-bc-test-2.txt)

$ cargo test -p vb_storage --lib --no-fail-fast step_succeeded_and_slot_written_record_kinds_are_distinct -- --nocapture
cargo test: 1 passed, 1534 filtered out (1 suite, 0.00s)
                                                (evidence/1782953003c-bc-test-3.txt)

$ cargo test -p vb_storage --lib --no-fail-fast legacy_envelope_id_12_with_step_succeeded_payload_is_accepted -- --nocapture
cargo test: 1 passed, 1534 filtered out (1 suite, 0.00s)
                                                (evidence/1782953003d-bc-test-4.txt)

$ cargo test -p vb_storage --lib --no-fail-fast canonical_id_33_round_trip_step_succeeded -- --nocapture
cargo test: 1 passed, 1534 filtered out (1 suite, 0.00s)
                                                (evidence/1782953003e-bc-test-5.txt)

$ cargo test -p vb_storage --lib --no-fail-fast slot_written_with_envelope_id_33_is_rejected -- --nocapture
cargo test: 1 passed, 1534 filtered out (1 suite, 0.00s)
                                                (evidence/1782953003f-bc-test-6.txt)
```

### 2 proptest files (PO-QXJGX-006, PO-QXJGX-007)

```
$ cargo test -p vb_runtime --test proptest_durability_matrix_step_succeeded
cargo test: 5 passed (1 suite, 0.16s)
                                                (evidence/1782953004-proptest-durability.txt)

$ cargo test -p vb_storage --test proptest_replay_summary_step_succeeded_split
cargo test: 4 passed (1 suite, 1.05s)
                                                (evidence/1782953005-proptest-replay.txt)
```

### Updated set row test (PRE-005 inverse)

```
$ cargo test -p vb_runtime --lib set_row_exists_and_is_correct -- --nocapture
cargo test: 1 passed, 1806 filtered out (1 suite, 0.00s)
                                                (evidence/1782953006-set-row-test.txt)
```

### Lint / format / check

```
$ cargo check -p vb_storage --features kani-vb-qxjgx-record-kind-split --tests
cargo build (1 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.07s
                                                (evidence/1782953002-cargo-check-vb_storage-feature.txt)

$ cargo clippy -p vb_storage --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo clippy: No issues found
                                                (evidence/1782953007-clippy-vb_storage.txt)

$ cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo clippy: No issues found
                                                (evidence/1782953008-clippy-vb_runtime.txt)

$ cargo clippy -p velvet-ballistics --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
cargo clippy: No issues found
                                                (evidence/1782953009-clippy-vb_cli.txt)

$ cargo check --workspace --all-targets --all-features
cargo build (5 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.30s
                                                (evidence/1782953010-cargo-check-workspace.txt)

$ cargo fmt --check -p vb_storage
(no output — formatting clean on my files)
                                                (evidence/1782953011-fmt-vb_storage.txt)
```

## Power-of-Ten Rules Affected

| Rule | Status | Notes |
|------|--------|-------|
| Rule 1 (simple control flow) | SATISFIED | `LegacyEnvelopeBinding` is a flat enum with a const constructor; no recursion or macro-hidden branching. |
| Rule 2 (fixed loop bounds) | N/A | No new loops introduced. |
| Rule 3 (no post-init alloc in critical paths) | N/A | The new binding is a value enum; no heap allocation. |
| Rule 4 (functions fit on one page) | SATISFIED | `for_journal_event` is 7 lines; `admits` is 6 lines; parity impl is 14 lines. All < 25 lines. |
| Rule 5 (assertion and invariant density) | SATISFIED | Invariants encoded in the enum (Exact vs Legacy { accepted_ids }) — illegal states (e.g. "StepSucceeded with no legacy tolerance") are unrepresentable. |
| Rule 6 (smallest scope) | SATISFIED | All bindings are scoped to a single match expression. |
| Rule 7 (checked returns) | SATISFIED | No new fallible results; `admits` returns bool. |
| Rule 8 (limited macros) | SATISFIED | No new macros. |
| Rule 9 (restricted pointer use) | SATISFIED | `accepted_ids: &'static [u16]` is a `&'static` reference; no raw pointers. |
| Rule 10 (warnings) | SATISFIED | `cargo clippy` exits 0 with `-D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock`. |

## Zero-Panic Rules Affected

| Forbidden construct | Status | Notes |
|---------------------|--------|-------|
| `unsafe` | SATISFIED | No `unsafe` introduced. Pre-existing `#![forbid(unsafe_code)]` at the top of every modified source file. |
| `unwrap` / `expect` | SATISFIED | No `unwrap` or `expect` introduced in production code. |
| `panic!` / `todo!` / `unimplemented!` / `unreachable!` | SATISFIED | No new panic paths. |
| Production `assert!` macros | SATISFIED | No new production asserts. The 18 pre-existing `assert!` calls in `flux_validation.rs:198-286` and `records.rs:331-349` are all inside `#[cfg(test)] mod ..._tests { ... }` blocks (test-only). |
| Unchecked indexing | SATISFIED | `accepted_ids.contains(&envelope_kind)` is bounds-safe. |
| Unchecked arithmetic | N/A | No arithmetic introduced. |
| Lossy `as` conversions | N/A | No `as` conversions introduced. |
| Ignored fallible results | SATISFIED | No new fallible results ignored. |

## Skipped Gates and Concrete Reasons

- **`cargo fmt --check -p vb_runtime`**: pre-existing `Diff in
  crates/vb_runtime/src/frame_pool/tests.rs:85, 114, 139` — file was NOT
  modified by this bead. Verified by `jj status` (not in modified list) and
  `jj diff --stat` (no entry). Pre-existing formatting debt at the
  `frame_pool/tests.rs` site. This is BLOCK_GLOBAL scope, not a regression
  introduced by this bead. Recorded as a residual risk.
- **`cargo kani <harness>` for the 5 new kani files**: BLOCKED_TOOLING
  (TBR-001). The workspace's `cargo kani list` path is blocked by a
  pre-existing unclosed-delimiter error in
  `crates/vb_core/src/frame/parts/kani_helpers.rs:22:7` (the
  `frame_kani_harnesses` module's `step_state_from_u8` helper). This
  pre-existing issue is in the parent commit `ywnswumt 1b72c500` (verified
  by `KANI_FEATURES=kani-vb-qxjgx-record-kind-split bash scripts/kani-list.sh
  vb_storage`). Not caused by this bead. The 5 kani files compile cleanly
  under `cargo check --features kani-vb-qxjgx-record-kind-split --tests` (see
  evidence/1782953002). Per proof-writer-report.md §Blockers, the kani
  harness execution is BLOCKED until TBR-001 is repaired by its owner in a
  separate bead. Not a regression of this bead.
- **Pre-existing `proptest_admission_with_budget_has_runtime_capacity_rejection_surface`
  in `crates/vb_core/tests/aggregate_resource_budget_properties_red.rs`**:
  fails because the proptest uses `include_str!("../../vb_runtime/src/admission.rs")`
  to test for the literal `admit_run_with_budget` text, but `admission.rs`
  itself only `include!`s the chunks; the function is in
  `admission/parts/chunk_006_admit_budget.rs`. Verified pre-existing by
  checking out parent commit `ywnswumt 1b72c500` and running the same test
  command — failure reproduces. Not caused by this bead. BLOCK_GLOBAL scope.
- **Workspace `cargo test --workspace --all-features`**: 1 pre-existing
  failure listed above. All other 16+ test suites pass. My bead introduces
  0 new failures; all 1678 vb_storage tests and 2348 vb_runtime tests pass.

## Residual Risks

1. **BLOCK_GLOBAL — pre-existing `kani_helpers.rs` unclosed delimiter**:
   the workspace's kani-harness execution is blocked workspace-wide (not
   scoped to vb-qxjgx). Out of scope for this bead. Owner: separate bead
   for `crates/vb_core/src/frame/parts/kani_helpers.rs`.
2. **BLOCK_GLOBAL — pre-existing `aggregate_resource_budget_properties_red`
   proptest failure**: out of scope for this bead. The proptest uses
   `include_str!` on `admission.rs` but the function is in an `include!`d
   chunk. Owner: separate bead to either move the function or relax the
   literal-string assertion.
3. **Pre-existing `cargo fmt` issues in `frame_pool/tests.rs`**: not touched
   by this bead. The diff stat confirms the file is unchanged.
4. **The 5 Kani harnesses + 2 proptest files remain PENDING_FORMAL_EXECUTION**
   in their `proof-evidence.md` records until the BLOCKED_TOOLING is
   repaired and the deep kani/proptest runs complete. The post-implementation
   state matches the proof-writer's expected post-fix surface (PO-QXJGX-001..007);
   the harnesses + proptest files compile cleanly under their feature gates.
   State 12 (formal-verifier) will execute the deep runs once TBR-001 is
   resolved.

## Performance Layer

No performance claim is made by this bead. The change:
- Replaces 1 OR-pattern match arm with 2 arms (no extra branches in the
  hot path; same number of comparisons).
- Adds a typed enum `LegacyEnvelopeBinding` (2 arms, 8 bytes including
  discriminant + 16-byte static slice reference) — strictly bounded
  allocation-free, fits in a register, no heap.
- The parity impl adds 1 const-fn call to `LegacyEnvelopeBinding::for_journal_event`
  and 1 `slice::contains` call for `Legacy { accepted_ids: &[12, 33] }`
  (O(2) bounded lookup).
- The `StepSucceeded` branch is the only one that pays the legacy cost;
  every other variant is `Self::Exact` (single equality compare).

No benchmark, profiler, or second-ring evidence is required because:
- No performance claim is made.
- The change is functionally-equivalent to the pre-fix surface (same
  number of match arms in the same positions; one new const call in the
  `JournalEvent` parity path; no allocation; no syscall; no lock).
- The parity gate is on the untrusted-input boundary, not a hot path;
  correctness dominates throughput.

## Post-Implementation Surface (verified on disk)

| Production symbol | Post-fix surface |
|-------------------|------------------|
| `RecordKind::StepSucceeded` | NEW arm at `records.rs:195`, wire id `33` |
| `RecordKind::id()` | NEW arm `Self::StepSucceeded => 33` at `records.rs:247` |
| `JournalEvent::record_kind()` | SPLIT at `events.rs:406-407`: `StepSucceeded → StepSucceeded`, `SlotWrittenEvent → SlotWritten` |
| `is_known_record_kind(33)` | `true` (extended at `validation.rs:24`) |
| `validate_kind_family(MAGIC_JOURNAL_EVENT, 33)` | `Ok(())` (extended at `validation.rs:50`) |
| `validate_kind_family(MAGIC_SNAPSHOT, 33)` | `Err(...)` (unchanged) |
| `validate_kind_family(MAGIC_BLOB, 33)` | `Err(...)` (unchanged) |
| `LegacyEnvelopeBinding::for_journal_event(StepSucceeded)` | `Legacy { accepted_ids: &[12, 33] }` |
| `LegacyEnvelopeBinding::for_journal_event(SlotWrittenEvent)` | `Exact` |
| `LegacyEnvelopeBinding::for_journal_event(<other>)` | `Exact` |
| `EnforceKindParity::enforce_kind_parity(JournalEvent)` | `admits` via binding; rejects cross-binds (POST-007) |
| `validate_journal_event_record_kind(JournalEvent)` | `admits` via binding; rejects cross-binds (POST-007) |
| `DURABILITY_MATRIX[set,do,choose,for_each,parallel,collect,aggregate,repeat,wait,ask]` | All step-closing rows use `StepSucceeded` (13 substitutions) |
| `DURABILITY_MATRIX[finish]` | `RunFinished` (unchanged) |
| `CURRENT_SCHEMA_VERSION` | `1` (UNCHANGED — no schema bump) |

## Files Changed

- `crates/vb_cli/src/status.rs` (1 line) — display StepSucceeded correctly
- `crates/vb_runtime/src/durability_matrix.rs` (13 substitutions across 10 rows)
- `crates/vb_runtime/src/durability_matrix/tests.rs` (1 line) — set row test
- `crates/vb_storage/src/codec/flux_validation.rs` (4 lines) — literal-sync id 33
- `crates/vb_storage/src/codec/kind_parity.rs` (+65 lines) — `LegacyEnvelopeBinding`
- `crates/vb_storage/src/codec/mod.rs` (3 lines) — use `LegacyEnvelopeBinding`
- `crates/vb_storage/src/codec/tests/replay_integrity.rs` (10 lines) — kind 33 admit
- `crates/vb_storage/src/codec/validation.rs` (2 lines) — kind 33 in id sets
- `crates/vb_storage/src/events.rs` (1 line) — split OR-pattern
- `crates/vb_storage/src/kani_record_kind.rs` (-12 lines) — delete pre-fix harness
- `crates/vb_storage/src/lib.rs` (1 line) — comment cleanup
- `crates/vb_storage/src/records.rs` (+10 lines) — `StepSucceeded` variant
- `crates/vb_storage/src/tests.rs` (1 line) — all-variants projection test

Total: 13 files changed, 120 insertions, 50 deletions.
