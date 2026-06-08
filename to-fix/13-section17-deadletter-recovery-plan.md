# Section 17 Dead-Letter Recovery Plan (Round 4)

**Scope:** Add production code paths for the 11 Section 17 runtime error codes
that Round 4 confirmed are defined in `velvet-ballistics-MASTER.md` §17 (lines
714-749) but are absent from the runtime `runtime_code()` surface, plus fix the
self-laundering parity tests that lock the gap in.

**Author:** Remaining-work mapping agent
**Inputs verified:**
- `velvet-ballistics-MASTER.md:714-749` (Section 17 golden list)
- `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs:35-50`
  (UNMAPPED bucket locking the gap)
- `crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs:159-217`
  (golden MAPPED/UNMAPPED/PARTIALLY_MAPPED classification)
- `crates/vb_runtime/src/error/mod.rs` (RuntimeError enum, 37 variants)
- `crates/vb_runtime/src/error/diagnostics.rs:96-149` (runtime_code emitter)
- `crates/vb_core/src/errors.rs:688-716` (CoreError::runtime_code)
- `crates/vb_storage/src/error/codes.rs:80-134` (JournalError codes, including the
  SecretUnavailable → ARTIFACT_MALFORMED misrouting at line 124)
- `crates/vb_cli/src/replay.rs:22-118` (cmd_replay → returns StorageError on
  `RecoveryError::ReplayDivergence`; should be ReplayDivergence/8)

**Round 4 verdict recap:** 4 of 11 are SHIP-BLOCKER:
1. `SECRET_UNAVAILABLE` (security audit failure)
2. `REPLAY_DIVERGED` (monitoring/alerting broken)
3. `WAIT_TIMEOUT` / `ASK_TIMEOUT` (incident triage impossible)
4. `STEP_SKIPPED_REFERENCE` (silent semantic drift)

---

## Definition of Done

A bead is complete only when **all** of the following are true:

1. Production code path constructs the variant in a place that the existing
   error machinery can actually surface. No new variants; no #[allow(dead_code)]
   decorations; no `Result::ok().ok_or_else(...)` laundering.
2. `runtime_code()` returns the exact Section 17 string constant for the new
   variant (or for the existing CoreError/RuntimeError variant that now
   carries the production case).
3. The new behavior is covered by an integration test that executes the
   production code path end-to-end, not a unit test that constructs a literal
   variant. The test name MUST contain the Section 17 code name in snake_case
   so the reverse-parity test can find it.
4. The reverse-parity and coverage-report tests are rewired so they fail when
   any Section 17 code lacks a production source. The `UNMAPPED` bucket
   ceases to exist.
5. `cargo build --workspace` and `cargo test --workspace` are green; Kani
   harnesses for the new variants (where applicable) are added under
   `crates/vb_runtime/src/kani_*` behind the same feature-gate pattern
   documented in AGENTS.md.
6. The bead is closed in `bd` and the evidence file is cited in the bead
   description.

---

## Minimum Viable Subset (SHIP-BLOCKER-first)

Do these four before any of the others. Each one is independently shippable.

| Order | Code | Bead | Hours | Why it goes first |
|------:|------|------|------:|-------------------|
| 1 | `SECRET_UNAVAILABLE` | vb-13d2a | 3 | Security audit: false ARTIFACT_MALFORMED classification masks secret-leak incidents. Smallest surface. |
| 2 | `REPLAY_DIVERGED` | vb-13d2b | 4 | Fixes the only currently-observable bug (CLI returns exit 5 instead of 8). Includes deleting the dead `cmd_replay` in `storage.rs:266`. |
| 3 | `WAIT_TIMEOUT` + `ASK_TIMEOUT` | vb-13d2c | 6 | Two variants on the same timer-wheel site. Without these, `handle_timer` cannot tell apart "wait deadline elapsed" from "stale timer fire". |
| 4 | `STEP_SKIPPED_REFERENCE` | vb-13d2d | 8 | Touches the `frame::mark_skipped` invariant and the `DriveFinished`/`Fail` terminal handoff. Highest risk. |

**MVS total: 21 hours (3 working days).**

After MVS is green, the remaining 7 codes are mechanical work, see
"Full Plan (11 items)" below.

---

## Test Infrastructure Plan (must land in the same PR as item 1)

### Self-laundering fix at `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs:35-50`

The `SECTION_17_UNMAPPED` constant and the
`section17_reverse_parity_unmapped_codes_have_no_sources` test together encode
"this code MUST NOT appear" — that is the exact anti-pattern the audit is
trying to prevent. The bucket exists *because* production code never
constructs the codes; it is a load-bearing lie.

**Replace with:**

```rust
// Reverse-parity test no longer carries a MAPPED/UNMAPPED split.
// It enumerates the full Section 17 golden list and asserts EVERY
// code has a runtime_code() source. A failure here is a Round-4-class
// dead-letter regression and is CI-fatal.

const SECTION_17_GOLDEN: &[&str] = &[
    "INPUT_MAPPING_FAILED",
    "INPUT_TYPE_MISMATCH",
    "SECRET_UNAVAILABLE",
    "REFERENCE_MISSING",
    "STEP_SKIPPED_REFERENCE",
    "ACTION_FAILED",
    "RETRY_EXHAUSTED",
    "WAIT_TIMEOUT",
    "ASK_TIMEOUT",
    "FOR_EACH_ITEM_FAILED",
    "TOGETHER_BRANCH_FAILED",
    "COLLECT_LIMIT_REACHED",
    "COLLECT_PAGE_FAILED",
    "REDUCE_ITEM_FAILED",
    "REPEAT_LIMIT_REACHED",
    "RESULT_REFERENCE_MISSING",
    "PAYLOAD_TOO_LARGE",
    "QUEUE_FULL",
    "IPC_FRAME_INVALID",
    "IPC_PAYLOAD_TOO_LARGE",
    "STORAGE_ERROR",
    "REPLAY_DIVERGED",
    "CONST_OUT_OF_BOUNDS",
    "MISSING_OUTPUT_SLOT",
    "STEP_STATE_OUT_OF_BOUNDS",
    "EXPRESSION_STACK_OVERFLOW",
    "EXPRESSION_STACK_UNDERFLOW",
    "INVALID_COMPILED_WORKFLOW",
    "INTERNAL_INVARIANT_VIOLATION",
    "UNSUPPORTED_PRIMITIVE",
    "BUDGET_EXCEEDED",
    "CAPABILITY_DENIED",
    "ADMISSION_DURABILITY_ERROR",
];

#[test]
fn section17_reverse_parity_every_golden_code_has_source() {
    // Build the union of all runtime_code() outputs from CoreError,
    // RuntimeError, IpcError, and any future error types by reading
    // the master §17 list.
    let mut all = BTreeSet::new();
    for code in collect_core_codes() { all.insert(code); }
    for code in collect_runtime_codes() { all.insert(code); }
    for code in collect_ipc_codes() { all.insert(code); }

    let missing: Vec<&str> = SECTION_17_GOLDEN
        .iter().copied().filter(|c| !all.contains(*c)).collect();
    assert!(missing.is_empty(),
        "Section 17 dead-letter regression: {:?} have no production runtime_code() source. \
         All 33 codes must be reachable from a production call site.", missing);
}
```

**Test refactor scope:**

- Delete `SECTION_17_UNMAPPED` constant (line 35-50).
- Delete `section17_reverse_parity_unmapped_codes_have_no_sources` test
  (line 213-239). Its assertion is now baked into the test above.
- Delete `UNMAPPED_CODES_WITH_RATIONALE` and `PARTIALLY_MAPPED_CODES` in
  `section17_runtime_code_coverage_report.rs:159-217` and the tests that
  iterate them. The coverage report collapses to a single
  "MAPPED = SECTION_17_GOLDEN" test that asserts presence.
- The 19/13/1 count test at line 250-277 must be updated to assert
  `MAPPED_CODES.len() == SECTION_17_GOLDEN.len() == 33`.

**Acceptance:** `rg "SECTION_17_UNMAPPED" crates/` returns zero matches.
The reverse-parity test fails on `cargo test` until every one of the 11
work items below lands; that is the desired property.

### Backing test helpers (no work duplication)

Add `crates/workspace_tests/tests/section17_test_helpers.rs` exposing:

```rust
pub fn assert_section17_code(source_kind: &str, error: &(impl RuntimeCode), expected: &str)
```

This helper is used by every per-item test below so the assertion shape is
identical. `source_kind` is a free-form label like `"RuntimeError::WaitTimeout"`
so a failure points at the production site, not at the helper.

**Risk:** low. Mechanical deletion of self-laundering test scaffolding.
**Hours:** 2 (helper module + 2 file rewrites + 1 review pass).
**Bead ID:** vb-13d2z (test infra). Must close in the same PR as item 1.

---

## Full Plan (11 Items)

Each item below is a standalone bead. The work item number is the order
in which they should be picked up after MVS.

---

### Item 1 — `SECRET_UNAVAILABLE` (SHIP-BLOCKER) — vb-13d2a

**Defect:** `vb_storage::JournalError::SecretUnavailable` (declared at
`crates/vb_storage/src/error/mod.rs:206-207`) is mapped to the
`ARTIFACT_MALFORMED_CODE` diagnostic code by the catch-all bucket in
`crates/vb_storage/src/error/codes.rs:113-124`:

```rust
Self::AdmissionRequired | Self::ArtifactInvalid { .. } | ...
| Self::SecretUnavailable              // <-- here
| Self::RunAlreadyExists               // <-- and here
... => Self::ARTIFACT_MALFORMED_CODE,
```

A security-classified failure ("a secret identifier required by the run
contract is not present in the secret store") is reported as
ARTIFACT_MALFORMED, which classifies into STORAGE_ERROR in
`runtime_code()`-space. An SRE cannot tell from the runtime code whether
they have a secret-rotation problem or a corrupt artifact problem. This
is exactly the audit trail Round 4 flagged.

**Fix (3 file changes):**

1. `crates/vb_storage/src/error/codes.rs:113-124` — split out
   `Self::SecretUnavailable` into its own arm returning a new
   `SECRET_UNAVAILABLE_CODE = DiagnosticCode::new(0x4040)`.
2. `crates/vb_storage/src/error/codes.rs:159-189` — same split in
   `symbolic_code()`: replace the bucket with a single
   `Self::SecretUnavailable => "SECRET_UNAVAILABLE"` arm.
3. `crates/vb_runtime/src/error/diagnostics.rs:96-149` — add a
   `RuntimeError::Core { source }` arm (or a new dedicated variant) that
   returns `Some("SECRET_UNAVAILABLE")` when
   `matches!(source.as_ref(), CoreError::SecretUnavailable { .. })`.
   Concretely, since the `vb_storage` path does not currently
   auto-promote to `CoreError`, the smallest change is to add a new
   `RuntimeError::SecretUnavailable` variant that mirrors
   `JournalError::SecretUnavailable`. The `From<JournalError>`
   conversion in `conversions.rs:13-19` matches on
   `JournalError::SecretUnavailable` and constructs
   `RuntimeError::SecretUnavailable`.

   This keeps the storage layer as the single producer of the error
   while exposing a stable runtime_code() surface.

**Production code path that constructs the variant:**

The existing `From<JournalError> for RuntimeError` impl at
`crates/vb_runtime/src/error/conversions.rs:13-19` is the construction
site. It currently routes every storage error to
`RuntimeError::StorageJournalAppend`. Update its body to:

```rust
impl From<vb_storage::JournalError> for RuntimeError {
    fn from(error: vb_storage::JournalError) -> Self {
        match error {
            vb_storage::JournalError::SecretUnavailable => Self::SecretUnavailable,
            other => Self::StorageJournalAppend { source: Arc::new(other) },
        }
    }
}
```

**Test:** `crates/vb_storage/src/error_tests.rs` — add
`secret_unavailable_diagnostic_code_is_distinct` asserting the new
`0x4040` code is not `ARTIFACT_MALFORMED_CODE` (line 233-236 must
flip). Then in `crates/vb_runtime/src/error/tests_basic.rs`, add
`runtime_error_secret_unavailable_emits_secret_unavailable_code` that
goes through the `From<JournalError>` route, not a hand-constructed
variant.

**Acceptance criteria:**

- `JournalError::SecretUnavailable.diagnostic_code() != ARTIFACT_MALFORMED_CODE`
- `JournalError::SecretUnavailable.symbolic_code()` returns the literal
  string `"SECRET_UNAVAILABLE"`.
- `RuntimeError::from(JournalError::SecretUnavailable).runtime_code() == Some("SECRET_UNAVAILABLE")`.
- The reverse-parity test sees `SECRET_UNAVAILABLE` in the union and
  passes for that code.
- The `secret_unavailable_error_code` test in
  `vb_storage/src/error_tests.rs:232-236` is updated to expect the new
  code; the old test value becomes a documented regression fixture.

**Risk:** Medium. The `ARTIFACT_MALFORMED` bucket covers nine variants
including `AdmissionRequired`, `ArtifactInvalid`, `InputTooLarge`, etc.
Splitting it requires verifying that no production test asserts the old
mapping for `SecretUnavailable` specifically. Search
`rg "ArtifactMalformed.*Secret\|Secret.*ArtifactMalformed" crates/` to
confirm; expected: zero hits.

**Hours:** 3.

---

### Item 2 — `REPLAY_DIVERGED` (SHIP-BLOCKER) — vb-13d2b

**Defect:** `RecoveryError::ReplayDivergence` is constructed 120+ times
across `vb_storage/src/recovery/` (see
`recovery/replay/summary.rs:126,132,454,460,630` and
`recovery/types.rs:124`). It surfaces from
`vb_storage::recovery::recover_full_journal` and is caught at
`crates/vb_cli/src/replay.rs:101-114`:

```rust
Err(e) => {
    // ...writes JSON or text error...
    return CliExitCode::StorageError.into();   // <-- exit code 5
}
```

`CliExitCode::StorageError == 5`, but the spec in
`crates/vb_cli/src/exit_code.rs:33-35` defines
`CliExitCode::ReplayDivergence == 8` for exactly this case. The
`vb_cli/src/commands_verify.rs:144` already maps
`VerifyError::ReplayDivergence(_)` to `CliExitCode::ReplayDivergence`,
proving the routing is the right one. The bug is a single missing
match arm.

A second, dead-copy defect: `crates/vb_cli/src/storage.rs:266-303`
defines a second `cmd_replay` that returns `ExitCode::FAILURE` for
every error and is never called from `dispatcher.rs:28`. The crate has
`#![allow(dead_code)]` at `lib.rs:2` so it compiles, but it is a
confusion magnet.

**Fix (2 file changes + 1 dead-code removal):**

1. `crates/vb_cli/src/replay.rs:101-114` — split the error match:
   match `RecoveryError::ReplayDivergence { step, detail }` first and
   return `CliExitCode::ReplayDivergence.into()`, then match other
   `RecoveryError` variants and return `CliExitCode::StorageError.into()`
   only for the storage-shaped ones.
2. `crates/vb_cli/src/replay.rs:48-83` — when `events.len() == 0` or
   the recovery produces events but the recovered terminal kind is
   `Failed`, also surface `CliExitCode::ReplayDivergence`. Round 4
   verdict called out "monitoring/alerting broken": a divergent
   recovery that *happens* to return some events is still divergent.
3. `crates/vb_cli/src/storage.rs:266-303` — delete the duplicate
   `cmd_replay` and the import in `commands.rs:17`. (Lib-level
   `dead_code` allow stays — the call site already imports from
   `crate::replay`.)

**Production code path:** This is a routing fix, not a new producer.
The producer is `RecoveryError::ReplayDivergence` in storage; the
router is `cmd_replay` in the CLI. The wire is closed by ensuring the
runtime_code() surface also carries `REPLAY_DIVERGED`.

**Runtime code wiring:** `crates/vb_runtime/src/error/diagnostics.rs`
needs a new arm for `RuntimeError::Core { source }` so that when a
runtime error wraps a `RecoveryError::ReplayDivergence` (or its
runtime-bridged equivalent), it surfaces as `Some("REPLAY_DIVERGED")`.
The bridge is `vb_runtime::recovery::DurableFrameRecoveryBoundary::hydrate_run_frame`
(`crates/vb_runtime/src/recovery.rs:63-71`), which currently returns
`RuntimeError::InvalidRecoveryHydration` for unsupported state. Add a
new `RuntimeError::ReplayDivergence { source: Box<vb_storage::recovery::RecoveryError> }`
variant that maps to `Some(Self::REPLAY_DIVERGED_RUNTIME_CODE)`. The
`From<RecoveryError>` conversion in
`crates/vb_storage/src/recovery/types.rs` is the natural construction
site (already used at conversions.rs:33-54 for `ResumeError`).

**Test:**

- `crates/vb_cli/src/replay.rs` — add `cmd_replay_returns_divergence_exit_code_on_replay_failure`
  that drives a synthetic recovery to return
  `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail: "x".into() })`
  via a mock or by poisoning a real journal. Assert
  `u8::from(code) == 8`.
- `crates/workspace_tests/tests/cli_integration.rs` — extend the
  existing `replay` test set to cover the exit-code matrix
  (success=0, not-found=2, divergence=8, storage-error=5).
- `crates/vb_runtime/src/error/tests_basic.rs` — add
  `runtime_error_replay_divergence_emits_replay_diverged_code`.

**Acceptance criteria:**

- `vb replay --run-id <id>` against a journal that recovers
  successfully returns exit 0.
- `vb replay --run-id <id>` against a journal that diverges returns
  exit 8, not 5.
- `rg "cmd_replay" crates/vb_cli/src/` returns exactly one definition
  (in `replay.rs`).
- Reverse-parity test sees `REPLAY_DIVERGED` in the runtime_code()
  union and passes for that code.

**Risk:** Medium-low. The dead-code removal is safe because the
duplicate is never called. The exit-code split is safe because
`VerifyError::ReplayDivergence` already maps to 8 (proves the
contract). The biggest risk is the new `RuntimeError` variant — the
equality, display, diagnostics, and conversion files all need a new
arm; AGENTS.md forbids `#[allow(dead_code)]` for genuine new code
paths.

**Hours:** 4.

---

### Item 3 — `WAIT_TIMEOUT` + `ASK_TIMEOUT` (SHIP-BLOCKER, two variants) — vb-13d2c

**Defect:** `ShardCommand::TimerFired` is routed to
`Shard::handle_timer` at `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:79-114`.
On a clean timer fire, it advances the run and emits either
`WaitResolved` or (for Ask) nothing. On a **timeout** specifically —
i.e. a `WaitEvent { timeout_slot: Some(..) }` node whose deadline has
elapsed, or an `Ask { timeout_slot: Some(..) }` whose prompt deadline
has elapsed — the current code does nothing distinct from a normal
timer resume. The runtime code surface emits
`RuntimeError::InvalidTimerFire` only on a stale (mismatched
generation/deadline/kind) fire, not on a deadline that *did* elapse.

Incident triage cannot tell a wait-deadline from a stale-fire because
the journal event is the same and the error code is the same. This is
the "incident triage impossible" gap Round 4 called out.

**Fix (1 new variant pair, 1 dispatch branch, 1 production path):**

1. `crates/vb_runtime/src/error/mod.rs` — add two variants next to
   the existing `InvalidTimerFire`:
   ```rust
   WaitTimeout {
       run: RunId,
       step: StepIdx,
       deadline: Instant,
   },
   AskTimeout {
       run: RunId,
       step: StepIdx,
       deadline: Instant,
   },
   ```
   These are constructed when the timer wheel fires a deadline that
   was set by `await_timer` for a `WaitEvent`/`Ask` node with a
   `Some(timeout_slot)`. A bare `WaitUntil` (no timeout context)
   still resolves normally and does not produce either error.
2. `crates/vb_runtime/src/error/diagnostics.rs:96-149` — add
   `Self::WaitTimeout { .. } => Some("WAIT_TIMEOUT")` and
   `Self::AskTimeout { .. } => Some("ASK_TIMEOUT")`. Add
   `WAIT_TIMEOUT_RUNTIME_CODE` and `ASK_TIMEOUT_RUNTIME_CODE` string
   constants near the existing `QUEUE_FULL_RUNTIME_CODE`.
3. `crates/vb_runtime/src/error/display.rs` — add static messages:
   `WaitTimeout { run, step, deadline }` →
   `"wait timeout: run {run:?} step {step:?} deadline {deadline:?}"`;
   same shape for Ask.
4. `crates/vb_runtime/src/error/equality.rs` — new unit tags 38, 39
   for the two new variants.
5. `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:79-114` — the
   `handle_timer` function must learn whether the fired timer is a
   *timeout* (i.e. it is a `PendingTimerKind::Wait` whose wait
   originated from `WaitEvent { timeout_slot: Some(..) }`, or a
   `PendingTimerKind::Ask`) versus a *deadline* (WaitUntil). This
   requires carrying a `is_timeout: bool` flag on the
   `PendingTimer` struct in `crates/vb_runtime/src/shard/timer.rs:20-28`,
   defaulting to `false`. Set it in `await_timer` at
   `crates/vb_runtime/src/shard/transitions.rs:137-177` for
   `WaitEvent`/`Ask` paths.
6. `handle_timer` at `chunk_002.rs:101-114`: if the fired timer is a
   timeout (flag true), return
   `Err(RuntimeError::WaitTimeout { run, step, deadline })` or
   `Err(RuntimeError::AskTimeout { .. })` *before* advancing the run.
   The run should be left in `Resumable` state with the timer
   removed, so that a subsequent `Cancel` or `Resume` from the
   caller can decide what to do. Append a journal event
   `TimerExpired { run, step, kind: Timeout }` for replay
   determinism.

**Production code path:** A workflow that contains
`WaitEvent { event: <some-slot>, timeout_slot: Some(<5s-slot>) }` and
is then left to tick for 5 seconds without the event materializing
will, after the timer wheel's `fire_expired` runs, see the
construction site. Round 4 verdict's exact scenario.

**Test:**

- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs` —
  extend `red_wait_event_with_timeout_returns_timeout_outcome` so
  that the assertion checks for
  `Err(RuntimeError::WaitTimeout { run, step, .. })`.
- New test in the same file: `red_ask_with_timeout_returns_ask_timeout_outcome`
  (use a workflow with `Ask { prompt, timeout_slot: Some(..) }` and
  let the wheel fire without an AskAnswer arriving).
- New test in `crates/vb_runtime/src/error/tests_basic.rs`:
  `runtime_error_wait_timeout_emits_wait_timeout_code` and
  the Ask analog.
- New Kani harness at
  `crates/vb_runtime/src/kani_wait_ask_timeout_disambiguation.rs` (gated
  by `#[cfg(kani)]` per AGENTS.md `kani-shard-command-queue` pattern).
  Property: for any fired timer, if `is_timeout` is true, the runtime
  error variant is the timeout variant; if false, the run advances
  without error.

**Acceptance criteria:**

- A timeout-bearing workflow that runs out its deadline surfaces
  `WAIT_TIMEOUT` (or `ASK_TIMEOUT`) at the `RuntimeError` boundary.
- A bare `WaitUntil` still advances the run and never produces
  either error.
- The two new codes appear in the reverse-parity union.
- Existing tests that drive `WaitEvent`/`Ask` *without* a timeout
  continue to pass.

**Risk:** High. Three correctness-sensitive files (timer.rs,
transitions.rs, chunk_002.rs) need coordinated edits. The flag on
`PendingTimer` is a wire-format-visible change to the timer wheel
and the deterministic-replay `kani_vb_fzgdn_timer_harnesses.rs`
harnesses must be updated. The new journal event `TimerExpired`
needs a kind discriminator that survives replay.

Mitigation: ship behind `RuntimeError::is_timeout_timer` boolean on
the journal event and use it to short-circuit during replay
(`matches!(event, TimerExpired { kind: Timeout, .. })`). Replay must
NOT re-emit the WaitTimeout error on its own; that would double-count
incidents.

**Hours:** 6.

---

### Item 4 — `STEP_SKIPPED_REFERENCE` (SHIP-BLOCKER) — vb-13d2d

**Defect:** `STEP_SKIPPED_REFERENCE` is the Section 17 code for "a
step was skipped because the step it references (e.g. a `Jump` target,
a `Choose.otherwise`, a `TogetherJoin`) was not present in the
compiled workflow or pointed to a step that has been removed/skipped
itself." This is *not* a compile-time error — the compile path
already validates `Jump { target }` against `parts.nodes.len()` in
`crates/vb_core/src/workflow/validation.rs:346`. It is a *runtime*
drift: e.g. a workflow that was admitted with digest D, but at
runtime the step table has been reduced (or has grown) by some
out-of-band action, and the next step reference no longer resolves.

The current runtime response is to return
`RuntimeError::Core { source: CoreError::InvalidProgramCounter { step } }`
or `CoreError::StepStateOutOfBounds { step }`. Both emit
`STORAGE_ERROR` (or nothing) in the runtime_code() surface. The
distinct semantic of "we tried to follow a reference, but the
target had been skipped" is lost.

**Fix (1 new CoreError variant, 1 new runtime_code arm, 1 production
construction site):**

1. `crates/vb_core/src/errors.rs` — add a new variant:
   ```rust
   /// A step reference resolved to a step that was itself skipped.
   #[error("step reference targets skipped step: {target:?} from {from:?}")]
   StepSkippedReference {
       /// Step that performed the reference.
       from: StepIdx,
       /// Step that was supposed to be the target, but is Skipped.
       target: StepIdx,
   },
   ```
   plus a `STEP_SKIPPED_REFERENCE_RUNTIME_CODE` constant
   (alongside `STEP_STATE_OUT_OF_BOUNDS_RUNTIME_CODE`) and a
   `STEP_SKIPPED_REFERENCE_CODE = DiagnosticCode::new(0x140E)` (the
   next free code in the 0x14xx budget block).
2. `crates/vb_core/src/errors.rs:688-716` — add the
   `Self::StepSkippedReference { .. } => Some(Self::STEP_SKIPPED_REFERENCE_RUNTIME_CODE)`
   arm to `runtime_code()`.
3. `crates/vb_core/src/engine/step.rs` and `run_loop.rs` — the
   natural construction site is wherever a `Jump { target }` is
   followed and the `target` step's `StepState` is `Skipped`. Look
   for the `set_pc`/`step_once` traversal; when the pc target
   resolves to a Skipped step, return the new error instead of
   the generic `InvalidProgramCounter`.

**Production code path:** A workflow that compiles cleanly, is
admitted with digest D, then experiences an out-of-band step-skip
(marked by `frame::mark_skipped` at `crates/vb_core/src/frame.rs:415-417`
in a `CompiledNodeKind::ErrorHandler { .. }` or
`RetryCheck { policy_slot, exhausted, .. }` block) and then has
its next-step `Jump` or `Choose.otherwise` land on the now-Skipped
target. The audit scenario is plausible during retry/recovery.

**Test:**

- `crates/vb_core/src/engine/tests/integration_step_behavior.rs` —
  add `step_skipped_reference_runtime_code_is_step_skipped_reference`
  building a workflow with `Jump { target: SkippedStep }` and
  asserting the constructed `CoreError` is the new variant and its
  `runtime_code()` is the literal string.
- `crates/vb_core/src/frame/tests_and_verification.rs:778-784` —
  extend the `mark_skipped` test to confirm that subsequent
  `set_pc` to the skipped step returns the new error.
- Reverse-parity test sees `STEP_SKIPPED_REFERENCE` in the union.

**Acceptance criteria:**

- A workflow whose next-step reference targets a Skipped step
  surfaces `STEP_SKIPPED_REFERENCE` at the runtime boundary.
- A workflow whose next-step reference is just out of bounds still
  surfaces `STEP_STATE_OUT_OF_BOUNDS` (the existing behavior must
  not be lost).
- The new code appears in the reverse-parity union.

**Risk:** High. `step.rs` and `run_loop.rs` are the heart of the
deterministic engine. The Round 4 verdict said "silent semantic
drift" — that is exactly the property we are trying to make loud.
Mitigation: keep the existing `InvalidProgramCounter` and
`StepStateOutOfBounds` constructions as fallbacks; only construct
`StepSkippedReference` when the target's `StepState` is `Skipped`.
Test with Kani under `vb_core` `kani-step-skipped-reference` lane.

**Hours:** 8.

---

### Item 5 — `INPUT_MAPPING_FAILED` — vb-13d2e

**Defect:** Currently only produced by the CLI:
`crates/vb_cli/src/run_compiled.rs:99-104` defines
`InputMappingError` (DecodeFailed / SlotCountExceeded /
SlotIndexOutOfRange), and the constants at
`crates/vb_cli/src/constants.rs:55-60` and `run.rs:15-22` carry the
string `"INPUT_MAPPING_FAILED: ..."` per Round 4's audit. But the
runtime code surface never sees it because:

1. The error originates in the CLI (not in `vb_runtime` or
   `vb_core`), so it has no `runtime_code()` method.
2. The exit code path is wrong: `run.rs:132-137` returns
   `CliExitCode::CompileFailed` for input mapping errors, but they
   are runtime-shaped (they are about the input data the *user*
   supplied at submit time, not the compiled IR).

**Fix (1 new RuntimeError variant, 1 production path, exit code fix):**

1. `crates/vb_runtime/src/error/mod.rs` — add
   ```rust
   InputMappingFailed {
       kind: &'static str,  // "decode" | "slot_count" | "slot_index"
       expected: Option<u32>,
       actual: Option<u32>,
   },
   ```
2. `crates/vb_runtime/src/error/diagnostics.rs` — arm
   `Self::InputMappingFailed { .. } => Some("INPUT_MAPPING_FAILED")`
   plus an `INPUT_MAPPING_FAILED_RUNTIME_CODE` constant.
3. `crates/vb_runtime/src/shard/lifecycle/chunk_001_submit.rs` — the
   construction site is in `dispatch_submit_with_inputs` and
   `dispatch_submit_with_inputs_and_contracts`. The existing
   `map_runtime_inputs` at
   `crates/vb_cli/src/run.rs:184-206` lives in the CLI. Move the
   decode-and-validate logic into a new
   `vb_runtime::shard::helpers::validate_input_mapping(compiled,
   input_data)` that returns `Result<Box<[(SlotIdx, SlotValue)]>,
   RuntimeError>`. The CLI keeps its `InputMappingError` enum only
   as a thin shim that converts to `RuntimeError::InputMappingFailed`.
4. `crates/vb_cli/src/run.rs:132-137` — change the exit code from
   `CompileFailed` to a new `CliExitCode::InputMappingFailed = 9`
   (extend `CliExitCode` and its discriminant test).

**Production code path:** `cmd_run` /
`cmd_run_compiled` flow on user input. The runtime error is now
produced *inside* `vb_runtime` rather than in the CLI shim, so the
runtime_code() surface is unified.

**Test:**

- `crates/vb_runtime/src/error/tests_basic.rs` — add
  `runtime_error_input_mapping_failed_emits_input_mapping_failed_code`.
- `crates/vb_runtime/src/shard/lifecycle/chunk_001_submit.rs` —
  add `submit_with_inputs_returns_input_mapping_failed_on_decode_error`
  using a `postcard::from_bytes::<&[u8]>(b"not-postcard")` payload.
- `crates/vb_cli/src/main_tests.rs:478-503` — update existing
  `map_runtime_inputs_rejects_malformed_input_bin` to expect
  `Err(InputMappingError::DecodeFailed)` (CLI shim) and a new test
  that asserts the runtime boundary returns
  `RuntimeError::InputMappingFailed { kind: "decode", .. }`.

**Acceptance criteria:**

- A `vb run --input` with a non-postcard payload returns exit 9
  (the new `InputMappingFailed` exit code), not exit 3
  (CompileFailed).
- The error string in stderr is
  `INPUT_MAPPING_FAILED: input-bin decode failed`.
- The runtime_code() surface emits `INPUT_MAPPING_FAILED` from
  both `RuntimeError::InputMappingFailed` and the `Core { source:
  CoreError::InputMappingFailed }` chain.

**Risk:** Medium. Adding a new `CliExitCode` variant (u8 = 9) shifts
all enum discriminant tests in `exit_code.rs:91-186` and
`mode_activation_tests.rs:867`. Any external automation matching
against exit codes 0-8 must be updated; document the change in
`velvet-ballistics-MASTER.md` §13 (CLI exit codes).

**Hours:** 5.

---

### Item 6 — `FOR_EACH_ITEM_FAILED` — vb-13d2f

**Defect:** `for_each_start` / `for_each_next` /
`for_each_join` at `crates/vb_runtime/src/primitives/for_each.rs`
return `EngineError::InternalInvariantViolation` (with hand-typed
`reason: "..."` strings) for body-level failures, but the body
itself runs as ordinary step execution between the start/next/join
calls. When a body step fails (e.g. an `EvalExpr` errors on a
per-item expression), the failure surfaces as the body's
`CoreError` (e.g. `TypeMismatch` → `INPUT_TYPE_MISMATCH` in
runtime_code()). The *iteration context* — which item, which
index, which body step — is lost.

This is the "body error loses iteration context" defect Round 4
flagged. The runtime code `FOR_EACH_ITEM_FAILED` should be emitted
when a for-each body step fails, carrying enough context for an
incident to point at the offending item.

**Fix (1 new RuntimeError variant, 1 wrapping layer):**

1. `crates/vb_runtime/src/error/mod.rs` — add
   ```rust
   ForEachItemFailed {
       item_index: u32,
       body_step: StepIdx,
       source: Box<CoreError>,
   },
   ```
2. `crates/vb_runtime/src/error/diagnostics.rs` — arm
   `Self::ForEachItemFailed { .. } => Some("FOR_EACH_ITEM_FAILED")`.
3. `crates/vb_runtime/src/engine/execute.rs:54-61` —
   `handle_for_each_start` / `handle_for_each_next` currently
   return `EngineError` which is then wrapped into
   `RuntimeEngineError::Core(e)` and surfaced as the body's error.
   Add a wrapper at the *runtime* layer (in
   `shard/lifecycle/chunk_002.rs` `apply_terminal_failed` /
   `apply_drive_result`) that catches the body's `CoreError`,
   inspects the run state for the active for-each iterator, and
   wraps it as `RuntimeError::ForEachItemFailed { item_index,
   body_step, source }`. The `item_index` is read from the
   iterator slot's length (total input length minus current
   iterator length).

   A cleaner approach: have `execute_node_full` return
   `Result<RuntimeSignal, RuntimeEngineError>` with a new
   `RuntimeEngineError::ForEachItem { item_index, body_step,
   source }` variant that `apply_terminal_failed` converts to
   `RuntimeError::ForEachItemFailed`. Choose whichever path is
   shorter during implementation.

**Production code path:** A workflow that contains
`ForEachStart { input: <list-of-3>, item_slot, limit, body, done }`
where the body has an expression that fails on item 1. After the
body failure, `apply_terminal_failed` wraps the engine error with
the for-each context.

**Test:**

- `crates/vb_runtime/src/primitives/for_each/tests.rs` (the inline
  tests) — add `for_each_body_failure_carries_item_context`
  building a workflow with a body step that always errors.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_007.rs` —
  add `red_for_each_body_failure_surfaces_for_each_item_failed`
  at the shard layer.
- `crates/vb_runtime/src/error/tests_basic.rs` — assert
  `runtime_code()` of the new variant.

**Acceptance criteria:**

- A for-each body failure surfaces
  `RuntimeError::ForEachItemFailed { item_index: 1, body_step: ..,
  .. }` at the runtime boundary.
- The runtime_code() emits `FOR_EACH_ITEM_FAILED`.
- Existing for-each happy-path tests still pass.

**Risk:** Medium. The wrapping layer needs the run's current
for-each state, which lives in the iterator slot. The `item_index`
computation must be deterministic and replay-stable.

**Hours:** 5.

---

### Item 7 — `TOGETHER_BRANCH_FAILED` — vb-13d2g

**Defect:** Same shape as item 6, for the `Together` primitive
family at `crates/vb_runtime/src/primitives/together.rs`. Body
errors from a `TogetherBranch { branch: 2, entry, join, .. }` lose
the branch index and the accumulator context.

**Fix:** Mirror of item 6.

1. `crates/vb_runtime/src/error/mod.rs` — add
   ```rust
   TogetherBranchFailed {
       branch: u16,
       entry: StepIdx,
       source: Box<CoreError>,
   },
   ```
2. `crates/vb_runtime/src/error/diagnostics.rs` — arm emits
   `"TOGETHER_BRANCH_FAILED"`.
3. `crates/vb_runtime/src/engine/execute.rs:65-78` — wrap
   `handle_together_branch` failures at the runtime layer with the
   new variant, similar to item 6.

**Production code path:** A workflow with
`TogetherStart { branches: [step_a, step_b, step_c], .. }` where
step_b fails. After the failure, the runtime surfaces
`TogetherBranchFailed { branch: 2, .. }`.

**Test:**

- `crates/vb_runtime/src/primitives/together_tests.rs` — add
  `together_branch_body_failure_carries_branch_index`.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_007.rs` —
  add `red_together_branch_failure_surfaces_together_branch_failed`.
- `crates/vb_runtime/src/error/tests_basic.rs` — assert
  `runtime_code()`.

**Acceptance criteria:**

- Together-branch body failure surfaces with branch index.
- `TOGETHER_BRANCH_FAILED` appears in the runtime_code() union.
- Existing together tests pass.

**Risk:** Medium. Same wrapping concerns as item 6.

**Hours:** 5.

---

### Item 8 — `COLLECT_PAGE_FAILED` — vb-13d2h

**Defect:** The existing `CoreError::CollectPageOrderViolation` at
`crates/vb_core/src/errors.rs:381-393` is the right shape, but its
`runtime_code()` arm at `errors.rs:711` lumps it with
`CollectPageLimitExceeded | CollectItemLimitExceeded |
CollectTimeLimitExceeded` under `COLLECT_LIMIT_REACHED`. A
**page-order violation** is semantically distinct from a **limit
exceeded** — the former is a corruption signal, the latter is a
quota signal. Round 4 verdict called out that "page-order
violations fall through to STORAGE_ERROR" via the conversion path:
`RuntimeError::Core { source: CoreError::CollectPageOrderViolation
{ .. } }` matches the catch-all `STORAGE_ERROR_RUNTIME_CODE` arm
at `vb_runtime/src/error/diagnostics.rs:118-121`.

**Fix (no new variant needed, just a new arm):**

1. `crates/vb_core/src/errors.rs` — add a
   `COLLECT_PAGE_FAILED_RUNTIME_CODE` constant (no new variant
   needed; `CollectPageOrderViolation` is the right shape).
2. `crates/vb_core/src/errors.rs:688-716` — split out
   `Self::CollectPageOrderViolation { .. } => Some(Self::COLLECT_PAGE_FAILED_RUNTIME_CODE)`
   from the `COLLECT_LIMIT_REACHED` bucket.
3. `crates/vb_core/src/errors.rs` — also expose a diagnostic code
   `COLLECT_PAGE_FAILED_CODE = DiagnosticCode::new(0x140F)` for
   `Self::CollectPageOrderViolation { .. }` (it currently falls
   into the `COLLECT_PAGE_ORDER_VIOLATION_CODE` at 0x140B, which
   stays — the runtime code and the diagnostic code are two
   different axes).

**Production code path:** A collect pagination that observes an
out-of-order, duplicate, or stale page (the construction sites
are in `crates/vb_runtime/src/primitives/collect/state.rs:128,
148,158`). The page-order violation already raises
`EngineError::CollectPageOrderViolation`; the only change is
mapping that error to a distinct runtime code.

**Test:**

- `crates/vb_runtime/src/primitives/collect/tests.rs:3435-3617` —
  extend the existing three `CollectPageOrderViolation` tests so
  each one asserts the runtime_code() of the wrapped error is
  `Some("COLLECT_PAGE_FAILED")` (currently no test checks
  runtime_code() of the collected violation).
- `crates/vb_core/src/errors.rs` (the tests inline) — add
  `core_error_collect_page_order_violation_emits_collect_page_failed_code`.

**Acceptance criteria:**

- A `CollectPageOrderViolation { kind: Duplicate, .. }` surfaces
  `COLLECT_PAGE_FAILED` in the runtime_code() surface.
- The `COLLECT_LIMIT_REACHED` code is no longer emitted for
  page-order violations (preserved for the three limit variants).
- Reverse-parity test sees `COLLECT_PAGE_FAILED` in the union.

**Risk:** Low. The variant is already there; we are only splitting
an over-broad match arm.

**Hours:** 2.

---

### Item 9 — `REDUCE_ITEM_FAILED` — vb-13d2i

**Defect:** Same shape as items 6 and 7, for the `Reduce` family
at `crates/vb_runtime/src/primitives/reduce.rs`. Body errors
from a `ReduceNext { iterator_slot, accumulator, body, .. }`
lose the iteration context.

**Fix:** Mirror of item 6.

1. `crates/vb_runtime/src/error/mod.rs` — add
   ```rust
   ReduceItemFailed {
       item_index: u32,
       body_step: StepIdx,
       accumulator_taint: vb_core::Taint,
       source: Box<CoreError>,
   },
   ```
   The accumulator taint is needed so a downstream debugging
   tool can see what the reducer had before it failed.
2. `crates/vb_runtime/src/error/diagnostics.rs` — arm emits
   `"REDUCE_ITEM_FAILED"`.
3. `crates/vb_runtime/src/engine/execute.rs:143-159` — wrap
   `handle_reduce_next` failures at the runtime layer with the
   new variant, including reading the accumulator slot's taint
   for the diagnostic context.

**Production code path:** A workflow with
`ReduceStart { input: <list-of-3>, accumulator, initial, body,
done }` where the body step fails on item 2.

**Test:**

- `crates/vb_runtime/src/primitives/reduce_tests.rs` (or the
  inline test module) — add
  `reduce_body_failure_carries_item_index_and_accumulator_taint`.
- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_007.rs` —
  add `red_reduce_body_failure_surfaces_reduce_item_failed`.
- `crates/vb_runtime/src/error/tests_basic.rs` — assert
  `runtime_code()`.

**Acceptance criteria:**

- Reduce body failure surfaces with item index and accumulator
  taint.
- `REDUCE_ITEM_FAILED` appears in the runtime_code() union.
- Existing reduce tests pass.

**Risk:** Medium. Same wrapping concerns as item 6.

**Hours:** 5.

---

### Item 10 — `RESULT_REFERENCE_MISSING` — vb-13d2j

**Defect:** `CompiledNodeKind::Finish { result: SlotIdx }` at
`crates/vb_core/src/workflow/types.rs:706-709` is the run's
terminal node. It selects a slot whose value is the run result.
The validation layer at
`crates/vb_core/src/workflow/validation.rs:347` checks that
`result < parts.slot_count`, but at *runtime*, if the
referenced slot is uninitialized (e.g. because a previous node
that should have populated it was skipped via `ErrorHandler`),
the runtime's read fails with
`CoreError::SlotUninitialized { slot }` (which does not have a
runtime_code() arm at all — see `errors.rs:688-716`).

The semantic of "the slot the result references is missing" is
distinct from a generic "slot uninitialized" — the latter can
happen in the middle of a workflow and is expected, the former
is a terminal-state corruption signal.

**Fix (1 new CoreError variant, 1 production path):**

1. `crates/vb_core/src/errors.rs` — add
   ```rust
   /// A Finish node's result slot was not populated.
   #[error("result reference missing for {step:?} slot {slot:?}")]
   ResultReferenceMissing {
       step: StepIdx,
       slot: SlotIdx,
   },
   ```
   plus `RESULT_REFERENCE_MISSING_RUNTIME_CODE` and
   `RESULT_REFERENCE_MISSING_CODE = DiagnosticCode::new(0x1410)`.
2. `crates/vb_core/src/errors.rs:688-716` — arm emits
   `"RESULT_REFERENCE_MISSING"`.
3. `crates/vb_core/src/engine/run_loop.rs` (or wherever the
   `Finish` node is dispatched) — the construction site is
   where the `Finish { result }` slot is read at the terminal
   step. Currently that read returns `EngineError::SlotUninitialized`;
   wrap that case with the new variant.

**Production code path:** A workflow whose `Finish { result:
<slot-N> }` is reached but slot N was never written because a
preceding error-handler skipped the writer step.

**Test:**

- `crates/vb_core/src/engine/tests/integration_step_behavior.rs` —
  add `finish_with_unwritten_result_slot_emits_result_reference_missing`.
- `crates/vb_core/src/frame/tests_and_verification.rs` — confirm
  the new error surface.

**Acceptance criteria:**

- A `Finish { result }` with an unwritten slot surfaces
  `RESULT_REFERENCE_MISSING` at the runtime boundary.
- Existing Finish tests with written slots continue to pass.

**Risk:** Low-medium. The construction site is well-bounded
(terminal node dispatch).

**Hours:** 4.

---

### Item 11 — `RETRY_EXHAUSTED` — vb-13d2k (NOTE: bonus item, not in original 11)

**Note on scope:** The Round 4 audit's original 11-code list did
not include `RETRY_EXHAUSTED` explicitly in the user's prompt,
but it appears in the `SECTION_17_UNMAPPED` bucket
(`section17_runtime_code_reverse_parity.rs:39`) and in the
Section 17 master list (`velvet-ballistics-MASTER.md:725`). It is
a clear gap; the test infrastructure rewrite will move it into
the golden list along with the other 11. This item is included
here for completeness; the MVS does not depend on it, but it
should be picked up by the same test-infra PR.

**Defect:** A `RetryCheck { policy_slot, body, exhausted }` whose
policy has been exhausted at runtime currently returns
`EngineError::StepBudgetExhausted` or similar (the body ran
without producing a result), losing the retry-exhaustion semantic.

**Fix:** Add a new CoreError variant and wire it.

1. `crates/vb_core/src/errors.rs` — add
   ```rust
   #[error("retry exhausted at step {step:?}: attempts {attempts} >= max {max}")]
   RetryExhausted {
       step: StepIdx,
       attempts: u16,
       max: u16,
   },
   ```
   plus `RETRY_EXHAUSTED_RUNTIME_CODE` and
   `RETRY_EXHAUSTED_CODE = DiagnosticCode::new(0x1411)`.
2. `crates/vb_core/src/errors.rs:688-716` — arm emits
   `"RETRY_EXHAUSTED"`.
3. `crates/vb_core/src/engine/retry_math.rs` —
   `execute_retry_check` returns the new variant when the
   policy is exhausted.

**Production code path:** A workflow with
`RetryCheck { policy_slot, body, exhausted }` whose body has
exceeded the retry policy's max attempts.

**Test:**

- `crates/vb_core/src/engine/tests/integration_retry_behavior.rs`
  (or whatever the existing retry test file is named) — add
  `retry_exhausted_at_runtime_emits_retry_exhausted_code`.
- `crates/vb_runtime/src/error/tests_basic.rs` — assert the
  bridge from `CoreError::RetryExhausted` to runtime_code.

**Acceptance criteria:**

- An exhausted retry policy surfaces `RETRY_EXHAUSTED` at the
  runtime boundary.
- Existing retry tests with successful retries continue to pass.

**Risk:** Low.

**Hours:** 3.

---

## Total Work-Hour Estimate

| Item | Code | Bead | Hours |
|------|------|------|------:|
| Test infra | (rewire reverse-parity) | vb-13d2z | 2 |
| 1 | SECRET_UNAVAILABLE | vb-13d2a | 3 |
| 2 | REPLAY_DIVERGED | vb-13d2b | 4 |
| 3 | WAIT_TIMEOUT / ASK_TIMEOUT | vb-13d2c | 6 |
| 4 | STEP_SKIPPED_REFERENCE | vb-13d2d | 8 |
| 5 | INPUT_MAPPING_FAILED | vb-13d2e | 5 |
| 6 | FOR_EACH_ITEM_FAILED | vb-13d2f | 5 |
| 7 | TOGETHER_BRANCH_FAILED | vb-13d2g | 5 |
| 8 | COLLECT_PAGE_FAILED | vb-13d2h | 2 |
| 9 | REDUCE_ITEM_FAILED | vb-13d2i | 5 |
| 10 | RESULT_REFERENCE_MISSING | vb-13d2j | 4 |
| 11 | RETRY_EXHAUSTED (bonus) | vb-13d2k | 3 |
| **Total** | | | **52 hours** |

(6.5 working days at 8h/day, or 13 calendar days at 4h/day.)

**MVS only (items 1-4 + test infra):** 21 hours = 2.6 working days.

---

## Bead Creation Order

Create the beads via `bd create` in this order so dependencies are
recorded:

```
vb-13d2z  test-infra: rewire reverse-parity to fail on dead letters  (blocks 1-11)
vb-13d2a  P0 SHIP-BLOCKER: SECRET_UNAVAILABLE production path          (blocks nothing)
vb-13d2b  P0 SHIP-BLOCKER: REPLAY_DIVERGED production path            (blocks nothing)
vb-13d2c  P0 SHIP-BLOCKER: WAIT_TIMEOUT / ASK_TIMEOUT production path  (blocks nothing)
vb-13d2d  P0 SHIP-BLOCKER: STEP_SKIPPED_REFERENCE production path     (blocks 5,6,7,9)
vb-13d2e  P1: INPUT_MAPPING_FAILED production path                    (blocks nothing)
vb-13d2f  P1: FOR_EACH_ITEM_FAILED production path                    (blocks 9)
vb-13d2g  P1: TOGETHER_BRANCH_FAILED production path                  (blocks nothing)
vb-13d2h  P1: COLLECT_PAGE_FAILED production path                     (blocks nothing)
vb-13d2i  P1: REDUCE_ITEM_FAILED production path                      (blocks nothing)
vb-13d2j  P1: RESULT_REFERENCE_MISSING production path                (blocks nothing)
vb-13d2k  P2: RETRY_EXHAUSTED production path (bonus)                 (blocks nothing)
```

Suggested dependency graph (use `bd dep add`):

```
vb-13d2z (test infra) ── blocks ──> all 11
vb-13d2d (step skipped ref) ── blocks ──> vb-13d2f, vb-13d2g, vb-13d2i
                                   (because they share the runtime-wrapping pattern)
vb-13d2f (for_each) ── blocks ──> vb-13d2i (reduce)  (reduce wrapping reuses for_each helpers)
```

---

## File-by-File Change Manifest

| File | Items | Change |
|------|-------|--------|
| `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs` | test-infra | Delete UNMAPPED bucket, rewrite tests to use SECTION_17_GOLDEN |
| `crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs` | test-infra | Collapse to MAPPED-only test |
| `crates/workspace_tests/tests/section17_test_helpers.rs` (new) | test-infra | Add `assert_section17_code` helper |
| `crates/vb_storage/src/error/codes.rs` | 1 | Split SecretUnavailable out of ARTIFACT_MALFORMED bucket |
| `crates/vb_runtime/src/error/mod.rs` | 2,3,6,7,9 | New variants: SecretUnavailable, ReplayDivergence, WaitTimeout, AskTimeout, ForEachItemFailed, TogetherBranchFailed, ReduceItemFailed, InputMappingFailed |
| `crates/vb_runtime/src/error/diagnostics.rs` | 1,2,3,5,6,7,9,11 | Add runtime_code() arms for all new variants |
| `crates/vb_runtime/src/error/display.rs` | 2,3,6,7,9 | Add Display messages |
| `crates/vb_runtime/src/error/equality.rs` | 2,3,6,7,9 | Add unit tags |
| `crates/vb_runtime/src/error/conversions.rs` | 1 | Match on `JournalError::SecretUnavailable` |
| `crates/vb_runtime/src/error/tests_basic.rs` | 1-11 | New tests asserting runtime_code() of each new variant |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | 2,3 | Split handle_timer for timeout vs deadline; ReplayDivergence bridge |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001_submit.rs` | 5 | validate_input_mapping helper |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_007.rs` | 6,7,9 | New tests for ForEach/Together/Reduce body failures |
| `crates/vb_runtime/src/shard/lifecycle_tests/chunk_008.rs` | 3 | Extend with WaitTimeout/AskTimeout |
| `crates/vb_runtime/src/shard/timer.rs` | 3 | Add `is_timeout` flag to PendingTimer |
| `crates/vb_runtime/src/shard/transitions.rs` | 3 | Set is_timeout on WaitEvent/Ask timer registration |
| `crates/vb_runtime/src/engine/execute.rs` | 6,7,9 | Wrap body errors with iteration context |
| `crates/vb_runtime/src/primitives/for_each.rs` | 6 | (No body error code change; context is added at runtime layer) |
| `crates/vb_runtime/src/primitives/together.rs` | 7 | (No body error code change; context is added at runtime layer) |
| `crates/vb_runtime/src/primitives/reduce.rs` | 9 | (No body error code change; context is added at runtime layer) |
| `crates/vb_runtime/src/primitives/collect/tests.rs` | 8 | Extend page-order-violation tests with runtime_code() assertions |
| `crates/vb_core/src/errors.rs` | 4,8,10,11 | New variants: StepSkippedReference, ResultReferenceMissing, RetryExhausted; split out CollectPageOrderViolation; new constants |
| `crates/vb_core/src/engine/run_loop.rs` | 4,10 | New construction sites for StepSkippedReference and ResultReferenceMissing |
| `crates/vb_core/src/engine/retry_math.rs` | 11 | Construct RetryExhausted |
| `crates/vb_core/src/engine/tests/integration_step_behavior.rs` | 4,10 | New tests |
| `crates/vb_core/src/frame/tests_and_verification.rs` | 4 | Extend mark_skipped test |
| `crates/vb_cli/src/exit_code.rs` | 5 | Add CliExitCode::InputMappingFailed = 9 |
| `crates/vb_cli/src/run.rs` | 5 | Change exit code from CompileFailed to InputMappingFailed; convert CLI error to RuntimeError |
| `crates/vb_cli/src/run_compiled.rs` | 5 | (Mirror of run.rs change) |
| `crates/vb_cli/src/replay.rs` | 2 | Split error arm: ReplayDivergence → exit 8 |
| `crates/vb_cli/src/storage.rs` | 2 | Delete dead cmd_replay |
| `crates/vb_cli/src/commands.rs` | 2 | Remove dead import |
| `crates/vb_storage/src/recovery/types.rs` | 2 | Existing `From<RecoveryError>` (none yet) — add one that produces `RuntimeError::ReplayDivergence` |
| `crates/workspace_tests/tests/cli_integration.rs` | 2 | Extend replay exit-code matrix |
| `crates/vb_storage/src/error_tests.rs` | 1 | Update SecretUnavailable code test |

---

## Risk Register

| Risk | Items | Severity | Mitigation |
|------|------:|----------|------------|
| `CliExitCode` extension (u8=9) breaks external automation | 5 | Medium | Document in master §13, mark transition in `STATE.md` |
| New `RuntimeError` variants shift equality, display, and conversion files | 2,3,6,7,9 | Low | All 4 files have parallel arm patterns; mechanical |
| `is_timeout` flag on `PendingTimer` is wire-format-visible | 3 | High | Field defaults to `false`; existing serialization preserved; add replay-aware `TimerExpired { kind }` journal event |
| Wrapping body errors at runtime layer risks double-counting during replay | 6,7,9 | Medium | Wrap only when the underlying `CoreError` is the body's first-occurrence; replay reads the wrap directly from journal |
| Removing dead `cmd_replay` from `storage.rs` may be referenced by other tests | 2 | Low | `rg "storage::cmd_replay" crates/` should return zero hits before deletion; the import in `commands.rs:17` is the only consumer and is itself dead |
| Test-infra rewrite changes 33 test counts, may break CI badges | test-infra | Low | Update the count test (line 250-277) in same PR |

---

## Definition of Done (final recap)

1. The 12 beads above (test-infra + 11 items + bonus) are created
   in `bd`, with the dependency edges as documented.
2. The test-infra PR lands first and turns the reverse-parity test
   red. CI is red until items 1-11 land.
3. Items 1-4 (MVS) are merged. CI is green. The 4 SHIP-BLOCKER
   codes are reachable from production.
4. Items 5-11 are merged in any order. CI stays green. The
   reverse-parity test stays green.
5. `rg "SECTION_17_UNMAPPED" crates/` returns zero matches.
6. `bd list` shows all 12 beads as `closed`.
7. `cargo test --workspace` is green; Kani harness for
   `is_timeout` is added under the `kani-shard-command-queue`
   feature-gate (AGENTS.md pattern).
8. `velvet-ballistics-MASTER.md` §13 is updated to document the
   new `CliExitCode::InputMappingFailed = 9` exit code (item 5).
9. `STATE.md` and `BIG-ASS-TESTING-TO-FIX.md` are updated to
   record that the 11 dead-letter codes are no longer dead.

End of plan.
