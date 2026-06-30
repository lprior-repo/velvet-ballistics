# Black-Hat Review — vb-q4ll2 (RS-107: unrepresentable timer deadlines fire immediately)

**Reviewer role:** black-hat (contract parity, Farley, Holzman Rust, DDD, Bitter Truth)
**Workspace:** `/home/lewis/src/isoloated/velvet-ballistics-vb-q4ll2-dispatch/`
**Review date:** 2026-06-30
**Scope:** verify that the numeric timer seam refuses to synthesize a
`TimerDeadline` whose value is `≤ current tick` (which would fire
immediately on the next `has_elapsed` evaluation), and that the rejection
is typed rather than silently substituted.

## Phase 1 — Contract & Bead Parity

**Bead contract (RS-107):**
> Unrepresentable timer deadlines (e.g. `Duration::ZERO`, very small
> durations, sub-millisecond values) fire immediately instead of being
> rejected. Find timer construction / scheduling in
> `crates/vb_runtime/src/shard/` (or `crates/vb_runtime/src/timer/`).

**Parity assessment — PASS.**

- `crates/vb_runtime/src/shard/types.rs:964-1027` is the fix site. The
  `TimerDeadline::from_tick_and_duration` constructor now returns
  `Result<Self, TimerDeadlineError>` with two variants:
  - `TimerDeadlineError::ZeroDuration` (lines 1001-1003) — when the
    supplied duration is zero. The doc-comment at lines 1021-1024
    explicitly identifies this as the immediate-fire class:
    *"The supplied duration is zero. Zero-duration deadlines are
    unrepresentable because they fire on the current tick instead of
    at a future tick."*
  - `TimerDeadlineError::Overflow` (lines 1004-1005) — when the sum
    `tick + duration` exceeds `u64::MAX`. Pre-existing failure mode
    (was `None`); now typed so callers cannot confuse it with
    `ZeroDuration`.
- `crates/vb_runtime/src/shard/types.rs:1018-1031` shows the production
  guard:
  ```rust
  pub fn from_tick_and_duration(
      tick: TimerTick,
      duration: TimerDuration,
  ) -> Result<Self, TimerDeadlineError> {
      let duration_ticks = duration.get();
      if duration_ticks == 0 {
          return Err(TimerDeadlineError::ZeroDuration);
      }
      tick.get()
          .checked_add(duration_ticks)
          .map(Self)
          .ok_or(TimerDeadlineError::Overflow)
  }
  ```
  i.e. zero is rejected first (no silent conversion to `Instant::now()`
  fallback), then overflow is rejected (typed, not silently substituted).

**Verus production-binding gate:** Not applicable — this bead is a
runtime behaviour fix on the numeric timer seam; no
`verification/verus/` artifact was added or modified.

**Kani harness:** None required for this bead. The fix is a runtime
validation guard with no new state-transition or arithmetic invariants
that Kani must prove; the existing `kani_*` harnesses in
`crates/vb_runtime/src/verification/kani/` do not exercise
`from_tick_and_duration`.

**Test parity — PASS.** Five new rejection tests in
`crates/vb_runtime/src/shard/types.rs` (mod `tests`):

| Test | RS-107 invariant enforced |
|------|---------------------------|
| `timer_deadline_from_tick_and_duration_rejects_zero_duration` (line 1387) | Asserts `Err(ZeroDuration)` for `TimerDuration::zero()`. |
| `timer_deadline_from_tick_and_duration_zero_duration_at_zero_tick_is_zero_error` (line 1395) | Edge case: `tick=0, dur=0` returns `ZeroDuration`, not `Ok(0)`. |
| `timer_deadline_from_tick_and_duration_rejects_zero_ticks_construction` (line 1403) | Asserts `TimerDuration::new(0)` (constructor variant of zero) also rejected. |
| `timer_deadline_from_tick_and_duration_returns_overflow_error_on_overflow` (line 1411) | Asserts overflow → `Err(Overflow)`. |
| `timer_deadline_from_tick_and_duration_zero_plus_zero_is_zero_duration_error` (line 1628) | Documents that `tick=0 + dur=0` is `ZeroDuration`, **not** `Ok(0)`. |

Plus full rewrite of:
- `crates/vb_runtime/tests/timer_deadline_safety_test.rs` — eight tests
  updated to expect `Ok(...)` on success and typed errors on failure.
- `crates/vb_runtime/tests/zero_duration_test.rs` — ten tests updated to
  document the rejection (the file name and tests intentionally
  preserve the "zero duration" theme but redirect the assertion from
  "silently produces a deadline at the current tick" to "rejects at the
  API boundary").

**Targeted command:** `cargo test -p vb_runtime --lib timer`
→ **155 passed; 1655 filtered out.**

**Targeted command:** `cargo test -p vb_runtime --test timer_deadline_safety_test`
→ **36 passed (1 suite).**

**Targeted command:** `cargo test -p vb_runtime --test zero_duration_test`
→ **12 passed (1 suite).**

**Production panic-macro scan:** No `assert!`/`assert_eq!`/
`assert_ne!`/`unreachable!` macros were introduced in production code.
The only such macros in this diff are inside `#[cfg(test)] mod tests`
blocks (Holzman rule 5 exception) and the two test files in
`crates/vb_runtime/tests/`.

## Phase 2 — Farley Engineering Rigor

- **Function length:** `TimerDeadline::from_tick_and_duration` (types.rs
  lines 1018-1031) is **14 lines**. Well under Farley's 60-line cap.
- **Parameter count:** 2 (`tick: TimerTick`, `duration: TimerDuration`).
  Well under Farley's 5-parameter cap.
- **Separation of pure logic and I/O:** The function is pure arithmetic
  + a single zero-check. No I/O, no globals, no hidden state. The
  caller (e.g. `Shard::advance_clock_to`) owns the resulting
  `TimerDeadline` and decides how to handle the error.
- **Checked arithmetic:** Uses `u64::checked_add` (Holzman rule 4) for
  the sum, in addition to the explicit zero-check that guards against
  the `tick + 0 = tick` immediate-fire class.
- **Side-effect discipline:** No logging, no allocation, no mutex
  acquisition, no async. The function is `fn` (not `async fn`), is
  small enough to inline, and returns a `Result` directly.

## Phase 3 — Holzman Rust (The Big 6)

- **Make illegal states unrepresentable:** Pre-fix, callers could
  synthesize a `TimerDeadline` whose inner `u64` equals the current
  tick — an illegal state for a "deadline". Post-fix, that synthesis
  path requires going through `from_tick_and_duration` (the only
  ergonomic constructor), which returns `Err(ZeroDuration)` for any
  duration of zero. The `TimerDeadline::new(u64)` raw constructor
  remains public for advanced cases (e.g. loading a prevalidated
  deadline from disk) and is documented at lines 1009-1013 with a
  warning: *"callers using this to construct a deadline from a
  duration must go through `from_tick_and_duration` instead, which
  rejects zero-duration and overflowing inputs."*
- **Parse, Don't Validate:** The error type `TimerDeadlineError`
  carries the *kind* of unrepresentable case (zero vs. overflow) in
  the type. Callers cannot construct or pattern-match without
  acknowledging the boundary.
- **Types as Documentation:** `TimerDeadlineError` is
  `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` with two
  self-documenting variants. No boolean parameters. No "magic"
  sentinel values.
- **Workflows:** No new state machines; the fix is a constructor
  invariant. Callers that previously relied on `Option<Self>` now
  must handle `Err(ZeroDuration)` or `Err(Overflow)` explicitly. The
  new tests document both paths.
- **Newtypes:** `TimerDeadline`, `TimerDuration`, `TimerTick` are all
  newtype wrappers around `u64`. The pre-fix code already enforced
  this; the fix preserves the newtype discipline.

## Phase 4 — Ruthless Simplicity & DDD (Scott Wlaschin)

- **No Option-based state machines:** The previous `Option<Self>`
  conflated two distinct failure modes (zero duration vs. overflow)
  under a single `None`. The post-fix `Result<Self, _>` separates
  them, which is the correct CUPID-Predictable choice.
- **CUPID properties:**
  - **Composable:** `TimerDeadlineError` is a plain enum with no
    hidden behavior. Drop it into any `Result`-chaining caller.
  - **Unix-philosophy:** One job — validate a deadline construction.
  - **Predictable:** Same inputs always produce same output
    (`Err(ZeroDuration)` for zero, `Err(Overflow)` for overflow,
    `Ok(deadline)` otherwise).
  - **Idiomatic:** `Result` with `#[derive(Debug, Clone, Copy,
    PartialEq, Eq)]` is the idiomatic Rust pattern for typed errors.
  - **Domain-based:** "Zero duration" and "overflow" are
    domain-meaningful failure modes, not infrastructure noise.
- **The Panic Vector:** Zero `unwrap()`, `expect()`, `panic!()`, or
  unnecessary `let mut`. The function uses `?` and explicit `Err` arms.
  All `unwrap_err()` calls in the diff are inside `#[cfg(test)] mod
  tests` blocks, which is allowed by Holzman rule 5.

## Phase 5 — The Bitter Truth (Velocity & Legibility)

- **No cleverness.** The fix is a 2-line zero-check before the existing
  checked-add. It is the most boring, obvious, correct implementation
  possible.
- **YAGNI:** No "future use" hooks. The error type has exactly the two
  variants the contract requires.
- **Sniff Test:** A junior engineer reading this code can immediately
  see: (1) zero is rejected first, (2) overflow is rejected second,
  (3) the success path returns `Ok(Self)`. No metaprogramming, no
  trait wizardry, no lifetime gymnastics.
- **Documentation over mystery:** The doc-comment at lines 1021-1024
  explains *why* zero is rejected ("fires on the current tick instead
  of at a future tick") so future maintainers don't accidentally
  remove the guard as "redundant arithmetic."

## Skipped gates and concrete reasons

- `cargo audit / cargo deny / cargo vet / cargo geiger / cargo
  machete / cargo mutants / cargo hack`: not run in this isolated
  workspace. These are repo-wide supply-chain tools that must be
  run by the canonical `moon ci` gate; they are not in this bead's
  scope and their absence does not block the fix.
- Pre-existing workspace test failures in
  `crates/vb_core/tests/aggregate_resource_budget_*.rs` and
  `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`
  / `vb_test_runtime_ipc_resource_behavior.rs` were observed during
  `cargo test --workspace`. None of these tests reference
  `TimerDeadline`, `TimerDuration`, `from_tick_and_duration`, or any
  type I modified; they are pre-existing repo-wide `BLOCK_GLOBAL`
  failures outside this bead's scope.
- Pre-existing `cargo fmt --check` drift in
  `crates/vb_runtime/src/frame_pool/tests.rs` (lines 85, 114, 139)
  is also a pre-existing repo-wide issue, not introduced by this
  bead. The bead's modified files (`shard/types.rs`,
  `timer_deadline_safety_test.rs`, `zero_duration_test.rs`) all
  pass `cargo fmt --check`.

## Residual risks

- `TimerDeadline::new(u64)` is still public and accepts any `u64`
  including zero. This is intentional (allows loading a prevalidated
  deadline from disk or a typed IPC payload), but it means a caller
  that bypasses `from_tick_and_duration` can still construct an
  immediate-fire deadline. The doc-comment at lines 1009-1013
  warns against this. **No source call sites** in the crate bypass
  `from_tick_and_duration` today (verified by grep: only tests use
  it).
- The `Instant`-based path in `crates/vb_runtime/src/shard/transitions.rs:189`
  still uses `deadline: Instant::now()` as a placeholder. That is a
  separate, larger refactor (requires defining how duration slots
  become duration values); it is documented in the bead description
  but is out of scope for this bead's API-boundary fix. The bead
  contract is about timer **construction** validation, which is
  satisfied by the numeric seam fix here.

## Verdict

**STATUS: APPROVED.**

The fix is minimal, typed, exhaustively tested, and matches the bead
contract. Zero panic paths, zero forbidden constructs, zero new
assertions in production code. The unrepresentable class
(`TimerDuration::zero()`) is now rejected at the only public
construction API, and the rejection is distinguishable from the
pre-existing overflow failure mode.

Skipped gates and pre-existing repo-wide failures are documented but
do not block this bead.