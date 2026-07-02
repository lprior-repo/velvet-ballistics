# vb-cib14 implementation evidence

## Files changed

- `crates/vb_runtime/src/journal/chunk_002.rs` — added explicit
  `Resumed` arm in `boundary_storage_event`; added
  `convert_resume_timestamp` helper; made
  `STORAGE_EVENT_CLONE_COUNT` thread-local to prevent proptest /
  basic-test cross-thread race.
- `crates/vb_runtime/src/journal/tests/chunk_002.rs` — updated
  proptest timestamp range cap to chrono's representable upper
  bound (so `from_timestamp` returns `Some`); made
  `storage_event_resume_timestamp_conversion_total_over_u64`
  actually exercise the production helper with `Ok`-path and
  overflow-path boundary sentinels; updated all
  `STORAGE_EVENT_CLONE_COUNT.store(0, …)` / `.load(…)` call sites
  to use the thread-local `.with(|c| c.borrow()…)` access pattern;
  gated the `CHRONO_MAX_SECS` constant on the `vb-cib14` feature
  to silence the dead-code warning in the default build.
- `crates/vb_runtime/src/error/mod.rs` — added the
  `RuntimeError::ResumeTimestampOverflow { run, timestamp }`
  struct variant.
- `crates/vb_runtime/src/error/display.rs` — added a
  non-empty static Display message for the new variant.
- `crates/vb_runtime/src/error/equality.rs` — added
  `runtime_error_resume_field_eq` helper and wired it into
  `runtime_error_field_eq` so the new struct variant compares
  field-wise.
- `crates/vb_runtime/src/error/diagnostics.rs` — added
  `RESUME_TIMESTAMP_OVERFLOW_CODE` (0x2020) and wired it into
  `diagnostic_code()`; added a `None` arm in `runtime_code()`
  (no high-level runtime code for this variant).
- `.beads/vb-cib14/implementation.md` — this file.

## Behavior implemented

### C1 — Resumed Maps to RunResumed

`StorageRuntimeJournal::storage_event(RuntimeJournalEvent::Resumed { run, timestamp }, seq)` now returns

`Ok(JournalEvent::RunResumed { run, seq, timestamp: convert_resume_timestamp(run, timestamp)? })`

The new arm lives in `boundary_storage_event` (mirroring
`WaitScheduled → WaitScheduledEvent`). The pre-fix bug shape
(`Ok(JournalEvent::RunFailedEvent { .. })` at the catch-all) is
**not** removed by this bead; that is `vb-edvbj`'s
responsibility.

### C2 — Timestamp Conversion Is Total And Explicit

The helper `convert_resume_timestamp(run, timestamp)` is defined
in `chunk_002.rs`:

```rust
fn convert_resume_timestamp(
    run: RunId,
    timestamp: u64,
) -> RuntimeResult<chrono::DateTime<chrono::Utc>> {
    let secs = i64::try_from(timestamp)
        .map_err(|_| RuntimeError::ResumeTimestampOverflow { run, timestamp })?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .ok_or(RuntimeError::ResumeTimestampOverflow { run, timestamp })
}
```

- `i64::try_from(timestamp_u64)` rejects values exceeding `i64::MAX`.
- `chrono::DateTime::<Utc>::from_timestamp(i64_secs, 0)` rejects
  far-future values (chrono's representable upper bound is
  `8_210_266_876_800` seconds; values ≥ that bound return `None`).
- No `as i64`, no `unwrap`, no `expect`, no modular wrap, no
  silent clamp, no panic.

### C3 — Storage Dispatch Totality (Paired With vb-edvbj)

The `Resumed` arm is now an explicit arm in
`boundary_storage_event`. The catch-all `_ =>` arm of
`storage_event` still routes `Resumed` through
`boundary_storage_event`; inside the new function arm, the
exhaustive match continues to enforce the C3 contract
post-`vb-edvbj`.

### C4 — Single-Clone Invariant Preserved

`storage_event(Resumed, _)` still calls `clone_for_dispatch(&event)`
exactly once (in the top-level dispatcher `match &event`). The
proptest `storage_event_resumed_pass_through` (gated on
`vb-cib14`) and the explicit
`storage_event_clones_the_resumed_event_exactly_once_per_dispatch`
test (also gated on `vb-cib14`) both assert
`STORAGE_EVENT_CLONE_COUNT == 1` after one Resumed dispatch.

**Note:** the `STORAGE_EVENT_CLONE_COUNT` global was promoted
from `static AtomicUsize` to `thread_local! RefCell<AtomicUsize>`
to eliminate a cross-thread race that proptest 1.11 introduced
when running the gated proptest in the same test binary as the
pre-existing
`storage_event_clones_the_event_exactly_once_per_dispatch`
regression test. proptest executes cases on an internal worker
thread (verified via `eprintln!("thread = {:?}", std::thread::current().id())`:
basic test ran on `ThreadId(2)`, proptest on `ThreadId(7)`) and
the shared static counter was being clobbered between the two
tests' `store(0)` / `storage_event(…)` / `assert count == 1`
sequences. Promoting to thread-local restores the single-clone
invariant for both tests simultaneously. The counter remains
`#[cfg(test)]` only; production code is unaffected.

### C5 — Recovery/Replay Classifies RunResumed As Active

Unchanged by this bead. The downstream classifier
`event_to_lifecycle(JournalEvent::RunResumed) →
LifecycleState::Active` (at
`crates/vb_storage/src/journal/incident.rs:203`) and
`is_in_flight_or_completed(JournalEvent::RunResumed) → Ok(false)`
(at `crates/vb_storage/src/recovery/hydrate.rs:754`) already
produce the correct post-fix shape. The user-visible symptom
(a resumed run reported as `Failed`) is removed by this fix.

### C6 — Seq And RunId Pass-Through

The new arm passes `seq` through unchanged from the
`boundary_storage_event`'s `seq: EventSeq` parameter. The
proptest asserts `mapped_event.seq() == seq` and
`mapped_event.run_id() == run` for all generated inputs.

> **Dispatch contract note:** the femdation dispatch prompt
> specified `seq: EventSeq::new(0)` literally, but the approved
> contract (`.beads/vb-cib14/contract.md` C6 — Seq And RunId
> Pass-Through) and the approved proptests
> (`chunk_002.rs:561` and `:771`) both require `seq` to be
> passed through from the input `seq` parameter. Per operator
> decision, contract C6 wins and the mapper passes `seq`
> through unchanged.

### C7 — Public Error Surface Adds ResumeTimestampOverflow

`RuntimeError::ResumeTimestampOverflow { run: RunId, timestamp: u64 }`
is added as a struct variant. The variant:

- Carries both `run` and the original `timestamp: u64` for
  diagnostics.
- Is not a unit variant.
- The enum is already `#[non_exhaustive]`
  (`crates/vb_runtime/src/error/mod.rs:6`), so adding a struct
  variant is non-breaking.
- Has a non-empty `Display` implementation
  (`"resume timestamp overflow: u64 cannot be losslessly
  converted to DateTime<Utc>"`).
- Compares field-wise via a new
  `runtime_error_resume_field_eq` helper wired into
  `runtime_error_field_eq`.
- Has a dedicated diagnostic code
  `RESUME_TIMESTAMP_OVERFLOW_CODE = DiagnosticCode::new(0x2020)`.

## Parity scenarios covered

- `storage_event(Resumed { run: RunId(15), timestamp: 1_700_000_000 }, seq)`:
  produces `Ok(JournalEvent::RunResumed { run, seq, timestamp })`
  with `run_id() == run` and `seq() == seq`.
- `storage_event(Resumed { run, timestamp: 0 }, seq)`:
  same shape; `timestamp: DateTime::<Utc>::from_timestamp(0, 0)` is
  `1970-01-01T00:00:00Z`.
- `storage_event(Resumed { run, timestamp: u64::MAX }, seq)`:
  `Err(ResumeTimestampOverflow { run, timestamp: u64::MAX })`.
- `storage_event(Resumed { run, timestamp: CHRONO_MAX_SECS }, seq)`:
  `Err(ResumeTimestampOverflow { run, timestamp: CHRONO_MAX_SECS })`
  (chrono overflow at `8_210_266_876_800`).
- `storage_event(Resumed { run, timestamp: 8_210_266_876_799 }, seq)`:
  `Ok(RunResumed)` (last legal value).

## Power-of-Ten / Holzman Rust compliance

- **Rule 1 (simple control flow):** `convert_resume_timestamp` is
  a straight-line sequence; `boundary_storage_event` is an
  exhaustive `match` over `RuntimeJournalEvent`. No recursion, no
  panic-driven control flow, no macro-hidden branches.
- **Rule 2 (fixed loop bounds):** no loops introduced. The
  proptest uses a bounded `ProptestConfig::with_cases(65536)`.
- **Rule 3 (no post-init allocation in critical paths):**
  `boundary_storage_event`'s new arm allocates no
  `String`/`Vec`/`HashMap`/`Box`; `convert_resume_timestamp`
  returns a `DateTime<Utc>` by value (statically sized). The
  `JournalEvent::RunResumed` payload is the same `DateTime<Utc>`
  value that already existed in the pre-fix `JournalEvent` shape.
- **Rule 4 (function length):** `convert_resume_timestamp` is 5
  logical lines including signature; the new
  `boundary_storage_event` arm is 6 logical lines.
- **Rule 5 (invariant density):** the conversion path
  (`i64::try_from` + `from_timestamp` + `.ok_or`) makes the two
  failure modes explicit and bounded. No `debug_assert!` /
  `assert!` / `unreachable!` introduced in production-reachable
  code.
- **Rule 6 (smallest scope):** the `run: RunId` parameter is
  captured by value (it's `Copy`); the `timestamp: u64` is
  captured by value.
- **Rule 7 (checked returns):** `i64::try_from` and
  `from_timestamp` are both `Result`/`Option`-returning; the
  `.map_err` / `.ok_or` arms convert them into the typed
  `RuntimeError::ResumeTimestampOverflow { run, timestamp }` and
  propagate via `?`.
- **Rule 8 (limited macros):** the only macro used is
  `thread_local!` (test-only, no allocation/panic/loop hidden).
- **Rule 9 (no pointer/indirect call):** no `unsafe`, no raw
  pointer, no `dyn Trait`, no function pointer introduced.
- **Rule 10 (zero warnings):** `cargo build --all-targets --all-features`
  is warning-free; `cargo test --no-run` is warning-free.

## Command results

- `cargo test -p vb_runtime --lib storage_event` — PASS, 1
  passed / 1806 filtered out. See
  `.beads/vb-cib14/evidence/cargo-vb-runtime-storage_event.log`.
- `cargo test -p vb_runtime --lib --features vb-cib14 storage_event`
  — PASS, 6 passed / 1806 filtered out
  (`storage_event_clones_the_event_exactly_once_per_dispatch`,
  `storage_event_clones_the_resumed_event_exactly_once_per_dispatch`,
  `storage_event_resumed_emits_typed_runtime_error_variant`,
  `storage_event_resume_timestamp_conversion_total`,
  `storage_event_resume_timestamp_conversion_total_over_u64`,
  `storage_event_resumed_pass_through`). See
  `.beads/vb-cib14/evidence/cargo-vb-runtime-storage_event-feature.log`.
- `cargo test -p vb_runtime --lib runtime_journal_event_resumed_has_correct_timestamp`
  — PASS, 1 passed / 1806 filtered out. See
  `.beads/vb-cib14/evidence/cargo-vb-runtime-resumed-timestamp.log`.
- `cargo test -p velvet-ballistics-workspace-tests --test vb_test_runtime_resume_replay --features vb-cib14`
  — PASS, 3 passed / 0 filtered out
  (`resume_replay::resume_replay_state12_pending_marker`,
  `resume_replay::resume_replay_classification_proptest`,
  `resume_replay::resume_replay_legacy_bug_proptest`). See
  `.beads/vb-cib14/evidence/cargo-workspace-tests-resume-replay-feature.log`.
- `cargo test -p vb_runtime --lib` (default features) — PASS,
  1807 passed / 0 failed. See
  `.beads/vb-cib14/evidence/cargo-vb-runtime-full-default.log`.
- `cargo test -p vb_runtime --lib --features vb-cib14` — PASS,
  1812 passed / 0 failed. See
  `.beads/vb-cib14/evidence/cargo-vb-runtime-full-feature.log`.
- `cargo build -p vb_runtime --all-targets --all-features` — PASS
  (warning-free). See
  `.beads/vb-cib14/evidence/cargo-vb-runtime-build-all-features.log`.
- `cargo check -p velvet-ballistics-workspace-tests --features vb-cib14 --tests` — PASS.

## Blockers / residual risk

- `vb-edvbj` (the catch-all `RunFailedEvent` removal) is still
  pending. The two beads are STRONG-coupled for release; the
  current code keeps the catch-all in place and only adds the
  explicit `Resumed` arm. Once `vb-edvbj` removes the catch-all,
  the dispatch remains total.
- The pre-existing test fragility in
  `storage_event_clones_the_event_exactly_once_per_dispatch` was
  masked by a global static counter; the counter is now
  thread-local so the basic test and the gated proptest no
  longer race. This is a test-infrastructure change, not a
  production behavior change.
- The pre-existing failure
  `vb_qi37_4_2_strict_runtime_admission::given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
  in `velvet-ballistics-workspace-tests` is `BLOCK_GLOBAL` and
  pre-dates this bead (verified by running the same test against
  the parent commit `b2a2ee46`). Not introduced by this bead;
  recorded as residual risk per the holzman-rust contract.
- No performance claim was made; no benchmark/profiler
  evidence attached. The mapper is structurally simple
  (one `match` arm + a 5-line helper); no allocation, no
  clone, no I/O, no lock.
