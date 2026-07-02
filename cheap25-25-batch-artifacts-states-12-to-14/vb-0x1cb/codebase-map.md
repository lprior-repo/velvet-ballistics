# Codebase Map — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 2 (explore)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:30:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- status: scout packet ready

## Bead Description (verbatim)

> moon run :source-length --force passes test-integrity but fails ignored-fallible-results:
> ViolationFound DISCARD-004 at crates/vb_runtime/src/shard/transitions.rs:146 for Ok(_)|Err(_)=>{}.
> Test-writer did not edit production code. Need to bind the result of a fallible call (currently
> discarded with `let _ = ...` or similar) and surface the error via the runtime's diagnostic path.

## Current Scanner State (re-checked at scout time)

`bash scripts/check-ignored-fallible-results.sh` from the isolated workspace exits 0 with:

```
JustifiedException|DISCARD-006|crates/vb_runtime/src/shard/transitions.rs|line=199
JustifiedException|DISCARD-006|crates/vb_runtime/src/shard/transitions.rs|line=86
```

The bead description's literal pattern `Ok(_)|Err(_)=>{}` at line 146 is NOT present in the
current `transitions.rs` source. Current line 146 is `self.run_state_insert(run, state)?;`
inside the `await_action` error rollback path. The bead text therefore reads as a snapshot of
the violation that existed when the bead was filed and **does not match today's source**.

The actual current DISCARD-006 sources (allow-listed in `scripts/ignored-fallible-results.allow`)
are `let _ = self.run_state_insert(run, state);` at:

- `crates/vb_runtime/src/shard/transitions.rs:100` (in `finish_run`, line 86 is the `#[allow]`)
- `crates/vb_runtime/src/shard/transitions.rs:202` (in `fail_run_state`, line 199 is the `#[allow]`)

These best-effort rollbacks drop the secondary `RuntimeError` from `run_state_insert`. The
primary journal-append error is what is currently returned via `return Err(error);`. The bead's
substantive instruction — *bind the result, surface it via the runtime's diagnostic path* — is
addressed by repairing exactly these two `let _ = ...` callsites, not by hunting a
`Ok(_)|Err(_)=>{}` match arm that no longer exists.

## Scoped Files

### Production source (must be repaired)

- `crates/vb_runtime/src/shard/transitions.rs` — 215 lines
  - `impl Shard::apply(run, event) -> RuntimeResult<()>`  (lines 50–76) — single routing method
    for `RuntimeEvent` mutations; already clean.
  - `impl Shard::keep_run(run, state) -> RuntimeResult<()>` (lines 79–83) — uses `?`, clean.
  - `impl Shard::finish_run(run, state) -> RuntimeResult<()>` (lines 87–112)
    - **Violation site A**: line 100 `let _ = self.run_state_insert(run, state);`
    - Annotated by line 86 `#[allow(clippy::let_underscore_must_use)]`.
    - Function semantics: after `append_journal_event(RunFinished)` fails, restore the run
      state and propagate the journal error. Rollback Result is currently dropped.
  - `impl Shard::await_action(...)` (lines 115–151) — uses `?` for rollback at line 146 and
    line 149; line 146 propagates the rollback error, not a `let _ = ...`. Already clean.
  - `impl Shard::await_timer(...)` (lines 154–195) — uses `?` and `match`; no `let _ = ...`.
  - `impl Shard::fail_run_state(run, state) -> RuntimeResult<()>` (lines 200–214)
    - **Violation site B**: line 202 `let _ = self.run_state_insert(run, state);`
    - Annotated by line 199 `#[allow(clippy::let_underscore_must_use)]`.
    - Same pattern as `finish_run`: rollback after failed `append_journal_event(RunFailed)`.

- `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` (only `run_state_insert` for context)
  - `pub fn run_state_insert(&mut self, run_id, state) -> RuntimeResult<Option<RunState>>` (lines 323–330)
  - Returns `RuntimeError` from `reserve_run_state_slot` (slot exhaustion / capacity bounds).
  - Result signature: `RuntimeResult<Option<RunState>>`; the `Option` is the previous value, not an error.

- `crates/vb_runtime/src/error/mod.rs` — `RuntimeError` enum (lines 7–203).
  Variants relevant to surfacing a secondary rollback error:
  - `RuntimeError::Core { source: Box<vb_core::errors::CoreError> }` (line 34–37)
    — wraps a `CoreError`, can carry `InternalInvariantViolation { reason: &'static str }`.
  - `RuntimeError::StorageJournalAppend { source: Arc<vb_storage::JournalError> }` (line 39–42)
    — typed journal failure; carries source via `Arc`.
  - No dedicated `RuntimeError` variant for "secondary rollback failure". Adding one would
    require updating `equality.rs`, `display.rs`, `diagnostics.rs`, `tests_basic.rs`,
    `tests_diagnostics.rs`, and `tests_conversion_refinement.rs`. See Open Questions §3.

- `crates/vb_runtime/src/error/conversions.rs` (55 lines) — `From` impls.
  - `From<vb_core::errors::CoreError> for RuntimeError` → `Core { source }`.
  - `From<vb_storage::JournalError> for RuntimeError` → `StorageJournalAppend { source }`.

- `crates/vb_runtime/src/error/diagnostics.rs` (210 lines) — symbolic/diagnostic codes.
  - `RuntimeError::diagnostic_code()` (line 47) returns one of the `0x200x` constants.
  - `RuntimeError::symbolic_code()` (line 168) — UNKNOWN for new variants unless matched.
  - `RuntimeError::runtime_code()` (line 108) — coarse operator-facing strings.
  - `RuntimeError::Core { source } => Self::STORAGE_JOURNAL_APPEND_FAILED_CODE` for non-QueueFull
    sources (line 63). New Core variants must be matched explicitly to avoid silent
    diagnostic drift.

- `crates/vb_runtime/src/trace/event.rs` — `TraceEvent` enum, `#[non_exhaustive]`.
  - `TraceEvent::RunFinished { run }` and `TraceEvent::RunFailed { run }` are emitted by
    `finish_run` (line 107) and `fail_run_state` (line 208). A new variant for rollback
    failures would need a public API addition; NOT preferred over typed-error surfacing.

### Static gates

- `scripts/check-ignored-fallible-results.sh` (319 lines)
  - Self-tests exercise each DISCARD class fixture (lines 267–312). Exit code 2 on
    ViolationFound, 0 on clean, 3 on allow-file parse failure.
  - `validate_allow_file` (lines 21–52) requires `path|class|owner=...|expiry=...
    |follow_up=...|reason=...` per row; path must target `crates/*/src` or
    `xtask/src`; class must be one `DISCARD-*` literal.
  - `classify_line` (lines 115–149) detects DISCARD-001..006 patterns.
  - `scan_tree` (lines 151–238) emits `JustifiedException` when allow-listed, else
    `ViolationFound`.

- `scripts/ignored-fallible-results.allow` (4 lines, single entry)
  ```
  crates/vb_runtime/src/shard/transitions.rs|DISCARD-006|owner=holzman-rust|expiry=2026-12-31|follow_up=vb-ttki3|reason=best-effort rollback must drop the secondary Result; the primary journal-append error is what gets surfaced to the caller
  ```
  - `follow_up=vb-ttki3` is incorrect: `vb-ttki3` per `to-fix/wave4/agent-12-adhoc-kani-harness.md`
    is a separate CI issue ("`moon ci` after forced push"). After the repair, this row should
    be removed entirely; the allow ledger is not a permanent waiver.

- `scripts/check-test-integrity.sh` (12 lines) — wraps `scripts/check-test-integrity.rs`.
  - Source-length gate depends on `test-integrity` (`.moon/tasks/all.yml` line 258).
  - `test-integrity` is currently passing; this bead does NOT change that.

### Moon v2 wiring (read-only)

- `.moon/tasks/all.yml`
  - `ignored-fallible-results` (lines 75–85) inputs include
    `scripts/check-ignored-fallible-results.sh`, `scripts/ignored-fallible-results.allow`,
    `@globs(sources)`, `.moon/tasks/all.yml`. Removing the allow row invalidates
    `@globs(sources)` cache for any file in `crates/vb_runtime/src/shard/`; subsequent CI
    reruns must not skip that gate.
  - `lint-src` (lines 50–62) depends on `ignored-fallible-results`.
  - `source-length` (lines 252–267) depends on `test-integrity`.
  - `ci` (lines 270–290) depends on `lint-src` and `source-length`.

## Existing Tests

- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs:270`
  `finish_run_appends_run_finished_event_and_inserts_terminal_run` — happy-path only;
  no rollback-failure test for `finish_run`. Adjacent coverage:
  - `lifecycle_tests/chunk_005.rs:108` — RunNotFound tick test.
  - `lifecycle_tests/chunk_005.rs:173, 216, 226, 261, 429` — terminal-fence behavior.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_004.rs:240–319`
  `LegacyStepFailsJournal` pattern: rejects `StepSucceeded` append with
  `StorageJournalAppend(WriteLockPoisoned)`; asserts typed error surfaces and frame is
  unchanged. This is the canonical pattern to mirror for the rollback-failure tests.
- `crates/vb_runtime/src/shard/tests/chunk_029.rs:357–399`
  `runtime_ask_timer_append_failure_does_not_register_pending_timer` — confirms
  `await_timer` rollback path returns typed `StorageJournalAppend(QueueFull)` and does not
  insert the timer. Uses `RejectTimerScheduledJournal::shared(PendingTimerKind::Ask)`.
- `crates/vb_runtime/src/shard/tests/chunk_029.rs:378`, `chunk_013.rs:207`,
  `impl_parts/chunk_001.rs:236`, `error/tests_basic.rs:167`,
  `error/tests_diagnostics.rs:73,123`, `journal/chunk_003.rs:14,22,25` —
  demonstrate the canonical `RuntimeError::from(vb_storage::JournalError::...)`
  construction idiom for the rollback target.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_005.rs:266–298`
  is the only `finish_run` test; there is NO existing test for `fail_run_state` or
  rollback-failure in either `finish_run` or `fail_run_state`. Coverage gap.

## Verification Artifacts

- `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs:42–69`
  - `kani_ask_answer_append_before_insert` — proves `apply(AwaitTimer|AwaitAction)`
    transitions runtime state to `Resumable`. Uses `kani::any()` for `RunId`. No
    rollback test for `finish_run` or `fail_run_state` exists.
- `crates/vb_runtime/src/verification/kani/kani_ask_answer_lifecycle.rs:80–100`
  - `kani_ask_answer_append_failure_no_timer` — uses `append_journal_event` stub
    (`cfg(kani)`) at `impl_parts/chunk_001.rs:206` which returns `Ok(())` and never
    exercises the error path. The stub must remain unchanged because expanding it
    would mutate production code's kani-only fork and require re-binding proofs.
- `crates/vb_runtime/src/verification/flux/vb_y9d3v_action_ticket_refinements.rs:159`
  - `extern_spec` references `production body in transitions.rs:69` (the comment
    quotes the OLD line range). When transitions.rs is repaired, this comment line
    may become stale; no code change required for that file (comment-only drift).
- `verification/kani/`, `verification/flux/`, `verification/verus/`,
  `verification/loom/`, `verification/proptest/` — covered by the proof-planner and
  proof-writer downstream lanes.

## Runtime Diagnostic Codes (relevant subset)

`RuntimeError::diagnostic_code()` mappings:

- `StorageJournalAppend { .. }` → `0x2008` (`STORAGE_JOURNAL_APPEND_FAILED_CODE`).
- `Core { source: CoreError::QueueFull }` → `0x2001` (`QUEUE_FULL_CODE`).
- `Core { source: _ }` → `0x2008` (fallback to journal-append code).
- `InternalInvariantViolation` (on `CoreError`, `0x1309`) — mapped via
  `INTERNAL_INVARIANT_CODE`. A new `RuntimeError::Core { InternalInvariantViolation }`
  call site would map to `0x1309` via the Core → STORAGE_JOURNAL_APPEND fallback
  UNLESS the match arm is extended in `diagnostics.rs:61–64` to route explicitly.

## Open Questions

1. **Surface contract**: the bead says "surface the error via the runtime's diagnostic path".
   Two viable implementations:
   a. **`RuntimeError::Core { source: InternalInvariantViolation { reason } }`** — wraps the
      secondary error as an internal invariant violation. Requires extending
      `diagnostics.rs:61–64` to map to `INTERNAL_INVARIANT_CODE` (`0x1309`) and matching
      the variant in `symbolic_code()` and `runtime_code()`.
   b. **Push `TraceEvent` for observability + return the typed `StorageJournalAppend`
      primary error** — trace path is non-error. Current code already uses this pattern
      (line 107, 208). Cheaper, but the secondary error is still lost.

   Recommendation: option (a) requires the contract planner; option (b) is the test-writer's
   default. The bead description phrasing favors (a) — "bind the result" implies the error
   is returned to the caller, not just observed.

2. **Two callers, two rollback sites**: `finish_run` (line 100) and `fail_run_state`
   (line 202) both have the same pattern. Either both must change consistently or only
   one. The bead describes ONE violation site (line 146) but the actual best-effort
   rollbacks are at lines 100 and 202. If the repair touches only one, the other stays
   DISCARD-006 and the allow row remains — partial fix.

3. **Allow-file cleanup**: removing `scripts/ignored-fallible-results.allow:4` requires
   that BOTH `let _ = ...` sites be repaired in the same change. The `expiry=2026-12-31`
   and `follow_up=vb-ttki3` fields would also have to be re-evaluated; the latter is
   already known to point at the wrong bead.

4. **Kani impact**: the kani stub for `append_journal_event` (`impl_parts/chunk_001.rs:206`)
   returns `Ok(())`; rollback paths in `finish_run` / `fail_run_state` are therefore
   unreachable under kani. The repair does NOT need new kani harnesses for the rollback
   branches, but a proptest for "finish_run rollback when journal rejects RunFinished"
   may be required by the test-writer lane.

5. **Test integrity**: any new tests must live under
   `crates/vb_runtime/src/shard/lifecycle_tests/` or `crates/vb_runtime/src/shard/tests/`
   to avoid the source-length `tests.rs`/`*_tests.rs` skip list in
   `check-ignored-fallible-results.sh:62–72`. Adding `tests.rs` at the root would
   re-trigger the gate.

6. **Bead description drift**: the literal "DISCARD-004 ... Ok(_)|Err(_)=>{}" string in
   the bead description does not match any current line. Downstream agents should treat
   the bead as "fix the allow-listed fallible-result regression at
   transitions.rs:100/202", not as "find a match-arm violation at line 146". This
   drift is captured here so the contract lane does not chase a phantom.

## Recommended Downstream Owners

- **rust-contract**: design the secondary-error surface contract (Core wrap vs. new
  variant). Pick the diagnostic-code mapping in `diagnostics.rs`. Decide whether
  the contract is identical for `finish_run` and `fail_run_state`.
- **proof-planner**: plan Flux refinement for the new `RuntimeError::Core { source }`
  match arm in `diagnostics.rs`. Kani is N/A because the kani stub returns `Ok`.
  Proptest should prove the rollback error surfaces for BOTH `finish_run` and
  `fail_run_state`.
- **test-planner**: add two behavior tests mirroring `chunk_004.rs:240` pattern:
  - `finish_run_rollback_surfaces_internal_invariant_when_journal_rejects_runfinished`
  - `fail_run_state_rollback_surfaces_internal_invariant_when_journal_rejects_runfailed`
  Each must assert the typed error and the unchanged terminal-fence invariants.
- **holzman-rust**: edit `transitions.rs:100` and `:202` (and remove the
  `#[allow(clippy::let_underscore_must_use)]` annotations at lines 86 and 199, plus
  the allow row at `scripts/ignored-fallible-results.allow:4`).
- **black-hat-reviewer**: confirm the surface contract preserves the primary error
  (the journal-append error from `append_journal_event` is the one the caller must
  see), and the secondary error is reported as an additional invariant violation
  without masking the primary.

## Risk Tags

- `release-blocker` — source gate violation, blocks `lint-src` and downstream CI
- `verification` — Kani stub currently unreachable for rollback; proptest required
- `concurrency` — N/A (single-shard, sequential)
- `persistence` — journal-append ordering already correct (best-effort rollback is
  downstream of the durability decision)
- `public-api` — no public API change if `Core` wrap is chosen; new variant would
  trigger a `RuntimeError` enum addition and tests_basic/diagnostics churn
- `diagnostic` — `RuntimeError::diagnostic_code()` match arm may need to be extended
- `performance` — N/A; no hot path change
- `migration` — N/A
- `user-visible-behavior` — error message changes only when both journal and
  rollback fail in the same call (rare; requires slot exhaustion under failed
  journal append)

## Required Verifier Modes

- `verify-standard` (covers ignored-fallible-results + test-integrity)
- `moon :lint-src` (must remain green)
- `moon :source-length --force` (must pass with no `--force` re-run needed after)
- `cargo test -p vb_runtime --lib` (lifecycle_tests/chunk_005 + new proptest)
- `cargo kani -p vb_runtime` (no harness change required; stub unchanged)
- `cargo flux -p vb_runtime` (only if `diagnostics.rs` is touched for a new match arm)
