# Wave 1 / Agent 12 — ad-hoc YAML-grammar deep-dive

**Scope:** canonical YAML authoring schema for `vb_yaml` (parser) and
`vb_validate` (validator). Bugs in this chunk reference runtime, storage,
core value, and core frame concerns — not the YAML parser itself. The
YAML-specific columns (`aliases-correct`, `trigger-correct`) are N/A for
all 11 IDs; the verdicts below reflect source-level inspection of the
claimed fix path in each bead.

**Method:** for each bug ID, read the bead, locate the touched source,
map it to the spec section the bead cites, run a targeted test, and grade
the close reason against the current code on `main` (HEAD `7fe116841`).

## Cross-cutting findings (apply to the whole chunk)

### YAML parser state vs master spec
- `crates/vb_yaml/src/ast/parse_steps.rs:82-94` — aliases preserved per
  Section 10: `"save" => parse_set`, `"do" | "run" => parse_do`,
  `"foreach" | "for_each" => parse_foreach`. **No alias regression.**
- `crates/vb_yaml/src/ast/parse_steps.rs:53-65` — legacy names
  `"parallel"` and `"aggregate"` are intercepted **before** the
  `is_primitive` gate and produce `YamlError::LegacyPrimitive` (canonical
  replacement is `together` / `reduce`). This is intentional, kani-bound
  behavior verified by `kani_is_primitive_legacy.rs`. **Legacy-name
  rejection holds.**
- `crates/vb_yaml/src/ast/parse_trigger.rs:39` — `when.webhook` requires
  empty body (`{}`) per Section 9 master example. `crates/vb_yaml/src/ast/parse_trigger.rs:81-86`
  — `when.event.type` matches the Section 9 master example
  (`type: github.pull_request`). **`event.type` and `webhook: {}` schema
  intact.** Note: a stale docstring at parse_trigger.rs:89 mentions
  `when.event.name` in an error message but the field gate at line 81
  enforces `&["type"]` only — cosmetic, not a contract bug.
- `crates/vb_yaml/src/ast/parse_steps.rs:139-140` — `"parallel"` and
  `"aggregate"` are still listed in `reject_unknown_step_fields` so they
  do not double-fail with `UnknownField`. This is correct: the explicit
  intercept at lines 53-65 owns the rejection.
- `cargo test -p vb_yaml --lib --no-fail-fast` → **228 passed** (0.02s).
- `cargo test -p vb_validate --lib --no-fail-fast` → **836 passed** (0.11s).

### Workspace breakage discovered (NOT in this chunk, but blocks tests)
- `crates/vb_runtime/src/test_harness.rs:33-58` and `:63-88` define
  `pub(crate) fn iterator_state_in_slot` twice with identical signatures
  and identical bodies. The `vb_runtime --lib` test target fails to
  compile (`iterator_state_in_slot must be defined only once`).
  Consequence: every targeted `cargo test -p vb_runtime --lib <name>`
  fails at compile, even though `cargo check -p vb_runtime` succeeds.
  This blocks `cargo test` evidence for vb-tpbgl, vb-tqn41, vb-uuicv,
  vb-v2zef. All four are graded on source-only inspection of the fix
  path.

## Per-bug findings

| bug-id | pri | spec-section | source-fix | test | aliases-correct | trigger-correct | targeted-cmd | result | verdict | evidence |
|---|---|---|---|---|---|---|---|---|---|---|
| vb-tpbgl | P2 | §15 IR (not yaml) | `crates/vb_runtime/src/primitives/collect.rs:527-552` `collect_next` validates empty-page against terminal cursor; `accept_empty_collect_page` (lines 554-571) returns `InvalidCompiledWorkflow` when `state.cursor < state.item_count` | `cargo test -p vb_runtime --lib collect_next` | n/a | n/a | `cargo test -p vb_runtime --lib collect_next` | BLOCKED (test_harness.rs dup) | PATCHED (source-only) | collect.rs:537 `if current.is_empty()` → `accept_empty_collect_page` enforces cursor validation. Cannot run unit test. |
| vb-tqn41 | P2 | §17 runtime (not yaml) | `crates/vb_runtime/src/primitives/retry.rs:44-58` `RetryPolicy::try_new` rejects `max_attempts == 0` with `RetryPolicyError::ZeroMaxAttempts` (variant at retry.rs:124-126) | retry.rs has dedicated tests; lib compile blocked | n/a | n/a | `cargo test -p vb_runtime --lib ZeroMaxAttempts` | BLOCKED | PATCHED (source-only) | retry.rs:54-55 `if max_attempts == 0 { return Err(RetryPolicyError::ZeroMaxAttempts); }`. Cannot run unit test. |
| vb-tw2jd | P1 | n/a (architectural-drift) | test file `crates/workspace_tests/tests/vb_a0t1_source_length_gate_tests.rs:134` **does not exist** in tree; `find` returns 0 matches | test deleted | n/a | n/a | `cargo test -p velvet-ballistics-workspace-tests --test vb_a0t1_source_length_gate_tests test_full_source_length_pipeline` | test target absent | NOT-PATCHED | bead cites `crates/workspace_tests/tests/vb_a0t1_source_length_gate_tests.rs:134` which is not in HEAD `7fe116841`; current workspace_tests has no `*a0t1*` file |
| vb-u1ezv | P3 | §14 Core Types (not yaml) | `crates/vb_storage/src/types.rs:75-94` `EventSeq::new(value: u64)` is **still** an unchecked `pub const fn` that accepts `u64::MAX`; `MAX_ENCODABLE` constant **does not exist**; `EventSeq::try_new` **does not exist**; `types/seq.rs` does not exist | `cargo test -p vb_storage --lib EventSeq` returns 0 filtered | n/a | n/a | `cargo test -p vb_storage --lib EventSeq` | 0 filtered (no SC-002 regression test exists) | NOT-PATCHED | types.rs:78 `pub const fn new(value: u64) -> Self { Self(value) }` — original bug unchanged; close reason's wave-8 commit `7586b096f` was reverted/refactored out of main |
| vb-uo52e | P2 | §15 storage recovery | `crates/vb_storage/src/recovery/replay/summary.rs:301-318` `reject_workflow_digest_mismatch` **still** uses `.map_or(Ok(()), \|result\| result)` at line 317 — falls through to silent success when no `RunAccepted` is found | `cargo test -p vb_storage --lib reject` → 195 passed (test label filters `reject_workflow_digest_mismatch` invocations) | n/a | n/a | `cargo test -p vb_storage --lib reject_workflow_digest` | 195 passed, but tests pass even with original bug | NOT-PATCHED | summary.rs:317 — original `map_or(Ok(()), ...)` branch still present; expected fix would replace with `Err(ReplayDivergence {..})` |
| vb-uuicv | P1 | §15 retry math | `crates/vb_runtime/src/primitives/retry.rs:158-163` `RetryState::from_policy` **still** seeds `remaining: policy.max_attempts()` (not `saturating_sub(1)`); `retry.rs:307-338` `evaluate_retry` **still** uses `if state.remaining == 0` (not `<= 1`) — produces `max_attempts + 1` total attempts | `cargo test -p vb_runtime --lib evaluate_retry` | n/a | n/a | `cargo test -p vb_runtime --lib evaluate_retry` | BLOCKED (test_harness.rs dup) | NOT-PATCHED | retry.rs:161 (`remaining = policy.max_attempts()`) and retry.rs:317 (`if state.remaining == 0`) — neither of the two suggested fix shapes is present |
| vb-uvyi0 | P3 | §14 Core Types (not yaml) | `crates/vb_core/src/ids/mod.rs:153-164` `MaxAttempts::try_new(0)` returns `EngineError::InvalidRepeatState { reason: "max_attempts_cannot_be_zero" }` (new variant at `crates/vb_core/src/errors.rs:332-339` routing code `INVALID_REPEAT_STATE` 0x140E) | `cargo test -p vb_core --lib max_attempts` → **10 passed** | n/a | n/a | `cargo test -p vb_core --lib max_attempts` | 10 passed | PATCHED | commit `b8bd66b65` (bead vb-t153y); ids/mod.rs:163 returns `EngineError::InvalidRepeatState` exactly as bead requires. Refactored from `domain_values.rs` → `ids/mod.rs`. |
| vb-uxfl0 | P1 | §15 storage recovery | `crates/vb_storage/src/recovery/recover.rs:140-216` — all four public functions (`recover_runtime_summary`, `recover_runtime_summary_with_expected`, `recover_runtime_frame_seed`, `recover_run_admission`) **still** call `journal.events_for_run(run)`; no `events_for_run_full` reader exists; no explicit reject path for snapshotted runs | `cargo test -p vb_storage --lib recover_runtime` | n/a | n/a | `cargo test -p vb_storage --lib recover_runtime` | 0 filtered | NOT-PATCHED | recover.rs:144, 160, 199, 211 — all four call `journal.events_for_run`; `events_for_run_full` symbol not in tree |
| vb-v2zef | P2 | §15 trace (not yaml) | `crates/vb_runtime/src/trace.rs:87-116` `TraceRing::drain_for_run` now stages non-target events in `preserved: VecDeque` and pushes them back via `self.producer.push(event)` (lines 93-114) — preserves evidence | `cargo test -p vb_runtime --lib drain_for_run` | n/a | n/a | `cargo test -p vb_runtime --lib drain_for_run` | BLOCKED (test_harness.rs dup) | PATCHED (source-only) | trace.rs:93-114 — `preserved.push_back(event)` collects non-target, `producer.push(event)` re-stages them; original drop-on-other-run behavior is gone |
| vb-vbdco | P3 | §15 runtime engine | `crates/vb_runtime/src/engine/types.rs:91-145` `EvidenceCollector::push_*` methods return `Result<(), EngineError::EvidenceCapacityExceeded>`; `crates/vb_runtime/src/engine/property_tests.rs:25-65` asserts the variant | `cargo test -p vb_runtime --lib EvidenceCapacityExceeded` | n/a | n/a | `cargo test -p vb_runtime --lib EvidenceCapacityExceeded` | BLOCKED (test_harness.rs dup) but `cargo test -p vb_core --lib` 2131 passed | PATCHED | commits `d8221505b`, `3bbfa264d`, `cd2de4c41`; vb_runtime/src/engine/types.rs:91-145, drive.rs propagation at type system level |
| vb-vluny | P1 | §14 Core value (not yaml) | `crates/vb_core/src/action.rs:565-571` `validate_idempotency_key_ingredients` **still** uses `let Ok(slot_taint) = frame.read_taint(slot) else { i = ...; continue; };` — **silently skips** out-of-bounds / uninitialized slots and returns `Ok(())` when at least one slot reads successfully | `cargo test -p vb_core --lib validate_idempotency` | n/a | n/a | `cargo test -p vb_core --lib validate_idempotency` | 0 filtered (no CV-102 regression test label) | NOT-PATCHED | action.rs:565 — `let Ok(slot_taint) = ... else { continue; };` — original bug unchanged; expected fix would `return Err(IdempotencyViolation::MissingKey(_))` or equivalent |

## Alias / trigger cross-check (read-only, no bugs cite yaml sections)

- Aliases correct in `crates/vb_yaml/src/ast/parse_steps.rs:83-86`:
  `save → set`, `run → do`, `foreach → for_each`. ✓
- Legacy `parallel`, `aggregate` rejected at parse_steps.rs:53-65 with
  `LegacyPrimitive` error. ✓ (legacy-name rejection intact; kani harness
  in `kani_is_primitive_legacy.rs` matches.)
- Trigger schema in `crates/vb_yaml/src/ast/parse_trigger.rs:37-48`:
  accepts `manual`, `webhook` (empty body), `schedule.cron`, `event.type`;
  rejects `ipc`, `http` with `UnsupportedTrigger`. ✓ matches Section 9.
- Cosmetic docstring nit at parse_trigger.rs:89 (`"when.event.name"` in
  error message) — runtime path enforces `type` only (line 81); not a
  contract violation.

## Workspace blocker (outside chunk)

`crates/vb_runtime/src/test_harness.rs:33-58` and `:63-88` define
`pub(crate) fn iterator_state_in_slot` twice with identical bodies.
Symptom: `cargo test -p vb_runtime --lib` (and any `--test <name>` that
pulls lib) fails at compile with
`iterator_state_in_slot must be defined only once in the value namespace
of this module`. Affects `cargo test` evidence for vb-tpbgl, vb-tqn41,
vb-uuicv, vb-v2zef, vb-vbdco. Source-level evidence substituted.

## Counts

- Bugs checked: 11
- PATCHED (source + tests): **1** (vb-uvyi0)
- PATCHED (source-only, test compile blocked): **3** (vb-tpbgl, vb-tqn41, vb-v2zef)
- PATCHED (commit evidence, lib blocked): **1** (vb-vbdco)
- NOT-PATCHED: **5** (vb-tw2jd, vb-u1ezv, vb-uo52e, vb-uuicv, vb-uxfl0, vb-vluny)
- Alias violations still present: **0**
- Trigger schema mismatches still present: **0**
- Top NOT-PATCHED with one-line reason:
  1. **vb-uo52e** (SR-008): `reject_workflow_digest_mismatch` at summary.rs:317
     still `.map_or(Ok(()), ...)` — silently passes when no `RunAccepted`
     exists in the slice.
  2. **vb-u1ezv** (SC-002): `EventSeq::new` at types.rs:78 is still an
     unchecked `const fn` accepting `u64::MAX`; no `try_new` or
     `MAX_ENCODABLE` exists.
  3. **vb-vluny** (CV-102): `validate_idempotency_key_ingredients` at
     action.rs:565 still uses `let Ok(slot_taint) = ... else { continue; }`
     — silently skips unreadable key slots and returns `Ok(())`.
- File path written: `/home/lewis/src/velvet-ballistics/to-fix/wave1/agent-12-adhoc-yaml-grammar.md`
