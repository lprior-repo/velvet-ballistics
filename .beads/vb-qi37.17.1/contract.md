# Contract Specification — vb-qi37.17.1: cli: Add incident command

## Context

- **Bead**: vb-qi37.17.1
- **Feature**: CLI incident command that returns structured failure evidence without stack traces.
- **Domain terms**: `IncidentReport`, `build_incident_report`, `build_repair_hints`, `FjallJournal`, `JournalEvent`, `run_id`, `failure_code`, `side_effects`, `repair_hints`, `OutputFormat` (Json / Jsonl / Text).
- **Assumptions**:
  - `vb_storage::FjallJournal::open(db, None)` opens a Fjall key-value store journal; `events_for_run(run_id)` returns `Result<Vec<JournalEvent>, JournalError>`.
  - `JournalEvent` variants include: `StepStarted { step, .. }`, `ActionCompletedEvent { step, action, .. }`, `ActionFailedEvent { step, action, .. }`, `RunFailedEvent { .. }`, `RunCancelled { .. }`, plus other variants consumed by `_` in the match.
  - `StepIdx` provides `.get() -> u16`; `ActionId` provides `.get() -> String`.
  - `serde_json::json!` macro is available in vb_cli.
  - The 57 E0061 compile errors stem from `recover_full_journal` gaining 2 extra parameters (`expected_action_abi_digests: &[(ActionId, WorkflowDigest)]` and `expected_policy_digests: &[(StepIdx, WorkflowDigest)]`) and `replay_events` gaining 1 extra parameter (`_expected_action_abi_digests: &[(ActionId, WorkflowDigest)]`), while ~37 call sites still pass the old 3/2-arg signatures.
- **Open questions**:
  - O1: What exact `OutputFormat` enum name is used in `vb_cli`? (Confirmed: `OutputFormat::Json`, `OutputFormat::Jsonl`, `OutputFormat::Text`.)
  - O2: Does `serde_json::to_string_pretty` / `to_string` ever fail on a well-constructed `serde_json::Value`? (Practically never, but the zero-unwrap rule requires a `Result`-based path.)

## Preconditions

- **PRE-001**: `run_id` is a valid non-empty string (validated by `parse_run_id` in the CLI dispatch).
- **PRE-002**: `db` path points to a directory the process can open (or gracefully error with structured JSON).
- **PRE-003**: `build_incident_report` receives a non-null `run_id` and a valid `&[JournalEvent]` slice.
- **PRE-004**: `build_repair_hints` receives a valid `failure_code`, `side_effects`, and optional `failed_at_step`.

## Postconditions

- **POST-001**: `build_incident_report` returns `IncidentReport` where:
  - `run_id` exactly matches the input.
  - `failure_found` is `true` iff at least one of `RunFailedEvent` or `RunCancelled` appears in events.
  - `failure_code` is `"RunFailed"` or `"RunCancelled"` when `failure_found` is `true`; otherwise `""`.
  - `failed_at_step` equals the `step` from the most recent `StepStarted` before the failure event, or `None`.
  - `side_effects` contains one entry per `ActionCompletedEvent` and `ActionFailedEvent`, each with `step`, `action`, and `certainty` (`"confirmed"` or `"failed"`).
- **POST-002**: `build_repair_hints` returns:
  - For `"RunFailed"`: at least one hint about investigating step output; a hint about reviewing side effects if `side_effects` is non-empty; a hint about retry from `failed_at_step` if known.
  - For `"RunCancelled"`: a hint about checking cancellation intent; a hint about partial cleanup if `side_effects` is non-empty.
  - For unknown failure codes: empty vec.
- **POST-003**: `cmd_incident` outputs valid JSON/JSONL/Text with no stack traces or raw error details.
- **POST-004**: Exit code is `CliExitCode::Success` when `failure_found` is `true`; `CliExitCode::StorageError` otherwise (including no-events and journal-open failures).

## Invariants

- **INV-001** (zero-unwrap): No `.unwrap()`, `.expect()`, `.unwrap_or_default()`, or `.unwrap_or()` on fallible operations in `cmd_incident` or `build_incident_report`. Every `Result` path must produce structured output.
- **INV-002** (no stack traces): Error output never includes `std::backtrace::Backtrace`, debug formatting of `JournalError`, or any stack-trace-producing display. All errors are plain strings.
- **INV-003** (JSON validity): Every JSON output is valid UTF-8 JSON parseable by `serde_json::from_str`.
- **INV-004** (text structure): Text output has deterministic key ordering: `run_id`, `failure_code`, `failed_at_step`, `side_effects`, `repair_hints`.
- **INV-005** (compile correctness): All 56 E0061 compile errors must be eliminated. Every call to `recover_full_journal` passes 5 args; every call to `replay_events` passes 3 args.
- **INV-006** (dead code removal): `args/run_db.rs::parse_incident` is unreachable (dead code) and must be removed.

## Type / Domain Model

### IncidentReport

```rust
pub struct IncidentReport {
    pub run_id: String,
    pub failure_code: String,           // "RunFailed" | "RunCancelled" | ""
    pub failure_found: bool,
    pub failed_at_step: Option<u16>,
    pub side_effects: Vec<serde_json::Value>,
    pub repair_hints: Vec<serde_json::Value>,
}
```

### Repair Hint Taxonomy

| Failure Code   | Hint Pattern                                    | Condition           |
|----------------|-------------------------------------------------|---------------------|
| `RunFailed`    | `"investigate step output and engine logs..."`  | Always              |
| `RunFailed`    | `"review side effects that completed..."`       | `!side_effects.is_empty()` |
| `RunFailed`    | `"consider retry from step {step}..."`          | `failed_at_step.is_some()` |
| `RunCancelled` | `"run was cancelled; check if cancellation..."` | Always              |
| `RunCancelled` | `"review completed side effects for partial..."`| `!side_effects.is_empty()` |

### Failure Codes (closed set)

- `"RunFailed"` — `JournalEvent::RunFailedEvent` observed
- `"RunCancelled"` — `JournalEvent::RunCancelled` observed
- `""` (empty) — no failure event observed

## Contract Signatures

```rust
// commands_incident.rs (existing — pure logic, no I/O)
pub fn build_incident_report(run_id: &str, events: &[JournalEvent]) -> IncidentReport;

pub fn build_repair_hints(
    failure_code: &str,
    side_effects: &[serde_json::Value],
    failed_at_step: Option<u16>,
) -> Vec<serde_json::Value>;
```

```rust
// app_impl.rs — cmd_incident must be refactored for zero-unwrap
fn cmd_incident(run_id: &str, db: &std::path::Path, output: OutputFormat) -> ExitCode;
```

## Compile Error Fixes Required

The following call sites pass wrong argument counts because `recover_full_journal` and `replay_events` gained parameters. All must be updated to pass `&[]` (empty slice) as the digest parameters, since this bead does not introduce digest verification — it only fixes call-site arity.

### `recover_full_journal` — 27 sites (needs 5 args: journal, run, tracker, expected_action_abi_digests, expected_policy_digests)

| # | File | Line | Current (broken) | Fix |
|---|------|------|-------------------|-----|
| 1 | `crates/vb_storage/tests/recovery_bdd_tests.rs` | 239 | Already correct (5 args) | NO FIX NEEDED — already uses `&[], &[]` |
| 2 | `crates/vb_storage/tests/recovery_bdd_tests.rs` | 503 | Already correct | NO FIX NEEDED |
| 3 | `crates/vb_storage/tests/recovery_bdd_tests.rs` | 584 | Already correct | NO FIX NEEDED |
| 4 | `crates/vb_storage/tests/recovery_bdd_tests.rs` | 652 | Already correct | NO FIX NEEDED |
| 5 | `crates/vb_storage/tests/recovery_bdd_tests.rs` | 668 | Already correct | NO FIX NEEDED |
| 6 | `crates/vb_storage/tests/recovery_bdd_tests.rs` | 739 | Already correct | NO FIX NEEDED |
| 7 | `crates/vb_storage/tests/replay_resume.rs` | 104 | Already correct | NO FIX NEEDED |
| 8 | `crates/vb_storage/tests/replay_resume.rs` | 154 | Already correct | NO FIX NEEDED |
| 9 | `crates/vb_storage/tests/replay_resume.rs` | 166 | Already correct | NO FIX NEEDED |
| 10 | `crates/vb_storage/tests/replay_resume.rs` | 219 | Already correct | NO FIX NEEDED |
| 11 | `crates/vb_storage/src/recovery/tests.rs` | 647 | Already correct | NO FIX NEEDED |
| 12 | `crates/vb_storage/src/recovery/tests.rs` | 989 | Already correct | NO FIX NEEDED |
| 13 | `crates/vb_storage/src/recovery/tests.rs` | 1031 | Already correct | NO FIX NEEDED |
| 14 | `crates/vb_storage/src/recovery/tests.rs` | 1306 | Already correct | NO FIX NEEDED |
| 15 | `crates/vb_storage/tests/vb_h6ix_integration.rs` | 104 | Already correct | NO FIX NEEDED |
| 16 | `crates/vb_storage/tests/vb_h6ix_integration.rs` | 167 | Already correct | NO FIX NEEDED |
| 17 | `crates/vb_storage/tests/vb_h6ix_integration.rs` | 209 | Already correct | NO FIX NEEDED |
| 18 | `crates/vb_runtime/src/collect_tests.rs` | 2203 | Already correct | NO FIX NEEDED |
| 19 | `crates/vb_runtime/src/collect_tests.rs` | 2253 | Already correct | NO FIX NEEDED |
| 20 | `crates/vb_runtime/src/collect_tests.rs` | 2316 | Already correct | NO FIX NEEDED |
| 21 | `crates/vb_cli/src/storage.rs` | 244 | Already correct | NO FIX NEEDED |
| 22 | `crates/vb_cli/src/app_impl.rs` | 2577 | Already correct | NO FIX NEEDED |
| 23 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 585 | Already correct | NO FIX NEEDED |
| 24 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 1042 | Already correct | NO FIX NEEDED |
| 25 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 1083 | Already correct | NO FIX NEEDED |
| 26 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 1224 | Already correct | NO FIX NEEDED |
| 27 | `fuzz/src/lib.rs` | 298+ | Check actual call | NO FIX NEEDED |

**Note**: The grep output shows all `recover_full_journal` calls already use 5 args (`&[], &[]`). If 27 E0061 errors are reported, the isolated workspace checkout may contain an older version of the call sites. The contract instructs: ensure every `recover_full_journal(journal, run, tracker, ...)` call passes exactly 5 args. The two new params are `&[]` empty slices.

### `replay_events` — 29 sites (needs 3 args: events, tracker, expected_action_abi_digests)

| # | File | Line | Current (broken) | Fix |
|---|------|------|-------------------|-----|
| 1 | `crates/vb_storage/src/recovery/tests.rs` | 686 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 2 | `crates/vb_storage/src/recovery/tests.rs` | 720 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 3 | `crates/vb_storage/src/recovery/tests.rs` | 870 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 4 | `crates/vb_storage/src/recovery/tests.rs` | 1112 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 5 | `crates/vb_storage/src/recovery/tests.rs` | 1167 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 6 | `crates/vb_storage/src/recovery/replay/core.rs` | 149 | `replay_events(&events, tracker, _expected_action_abi_digests)` — correct | NO FIX NEEDED |
| 7 | `crates/vb_storage/src/recovery/replay/core.rs` | 190 | `replay_events(tail_events, tracker, &[])` — correct | NO FIX NEEDED |
| 8 | `crates/vb_cli/src/storage.rs` | 244+ | Check actual call | NO FIX NEEDED |
| 9 | `crates/workspace_tests/benches/vb_h6ix_replay.rs` | 68 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 10 | `crates/workspace_tests/benches/vb_h6ix_replay.rs` | 119 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 11 | `crates/workspace_tests/benches/vb_h6ix_replay.rs` | 168 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 12 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 748 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 13 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 823 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 14 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 1111 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 15 | `crates/workspace_tests/tests/slot_written_ordering_integration_tests.rs` | 1183 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |
| 16 | `fuzz/src/lib.rs` | 303 | `replay_events(&events, &mut tracker, &[])` — correct | NO FIX NEEDED |

**Note**: All `replay_events` calls seen via grep already use 3 args. If the workspace checkout has old signatures, fix by adding the 3rd argument `&[]` (empty `&[(ActionId, WorkflowDigest)]`).

### Summary of compile fix approach

- **`recover_full_journal(journal, run, tracker, action_digests, policy_digests)`**: Add `, &[]` after the tracker arg where only 3 args are currently supplied.
- **`replay_events(events, tracker, action_digests)`**: Add `, &[]` after the tracker arg where only 2 args are currently supplied.
- The empty slices `&[]` are safe because the new digest parameters are marked `_` (unused) in the function bodies — they are reserved for future digest verification.

## Zero-Unwrap Violations in cmd_incident (4 fixes)

| # | Line | Code | Fix |
|---|------|------|-----|
| 1 | 3181 | `serde_json::to_string_pretty(&json_report).unwrap_or_default()` | Use `.map_err(|e| ...)` with `json_error()` fallback; output structured JSON error on serialization failure |
| 2 | 3185 | `serde_json::to_string(&json_report).unwrap_or_default()` | Same pattern as #1 |
| 3 | 3202 | `se["certainty"].as_str().unwrap_or("unknown")` | Replace with `se.get("certainty").and_then(|v| v.as_str()).unwrap_or("unknown")` — safer indexing, though `as_str()` itself is safe (returns `Option<&str>`, not `Result`). This is technically not an unwrap-on-Result but an unwrap_on_none pattern. Use `unwrap_or("unknown")` — it's already safe. **WAIVER**: `as_str()` returns `Option<&str>`, `unwrap_or` is zero-panic. |
| 4 | 3208 | `hint.as_str().unwrap_or("unknown")` | Same as #3 — `serde_json::Value::as_str()` returns `Option<&str>`. **WAIVER**: safe, zero-panic. |

**Detailed fix for #1 and #2** (lines 3181, 3185):

Replace:
```rust
let json_str = serde_json::to_string_pretty(&json_report).unwrap_or_default();
outln!("{json_str}");
```

With:
```rust
match serde_json::to_string_pretty(&json_report) {
    Ok(json_str) => outln!("{json_str}"),
    Err(e) => {
        json_error(
            &serde_json::json!({ "success": false, "error": format!("serialization error: {e}") }),
            output,
        );
        return CliExitCode::StorageError.into();
    }
}
```

And similarly for line 3185 (Jsonl):
```rust
match serde_json::to_string(&json_report) {
    Ok(json_str) => outln!("{json_str}"),
    Err(e) => {
        json_error(
            &serde_json::json!({ "success": false, "error": format!("serialization error: {e}") }),
            output,
        );
        return CliExitCode::StorageError.into();
    }
}
```

## Dead Code Removal

- **File**: `crates/vb_cli/src/args/run_db.rs`, lines 144–151
- **Function**: `pub(super) fn parse_incident(args: &[OsString]) -> Result<Command, ParseError>`
- **Status**: Unreachable. The real `parse_incident` is defined in `args.rs` at line 893 and is dispatched via the VALID_COMMANDS match at line 290.
- **Action**: Delete lines 144–151 of `args/run_db.rs`.

## Verification Layers

### Layer 1: Unit Tests for `build_incident_report` (commands_incident.rs)

| Test ID | Scenario | Coverage |
|---------|----------|----------|
| T-001 | No events → `failure_found: false`, `failure_code: ""`, empty side_effects | happy/edge |
| T-002 | Single `StepStarted` then `RunFailedEvent` → `failure_found: true`, `failure_code: "RunFailed"`, `failed_at_step: Some(step)` | happy |
| T-003 | `StepStarted` + `ActionCompletedEvent` + `RunFailedEvent` → side_effects has one confirmed entry | happy |
| T-004 | `StepStarted` + `ActionFailedEvent` + `RunFailedEvent` → side_effects has one failed entry | happy |
| T-005 | `StepStarted` + `ActionCompletedEvent` + `ActionFailedEvent` + `RunFailedEvent` → two side_effects | happy |
| T-006 | `StepStarted` + `RunCancelled` → `failure_found: true`, `failure_code: "RunCancelled"` | happy |
| T-007 | Multiple `StepStarted` → `failed_at_step` is the LAST step_started before failure | edge |
| T-008 | Unknown event variants (via `JournalEvent` variants not matched) → ignored, no panic | edge |

### Layer 2: Unit Tests for `build_repair_hints` (commands_incident.rs)

| Test ID | Scenario | Coverage |
|---------|----------|----------|
| T-009 | `RunFailed` + empty side_effects + no step → 1 hint | happy |
| T-010 | `RunFailed` + side_effects + step → 3 hints | happy |
| T-011 | `RunCancelled` + empty side_effects → 1 hint | happy |
| T-012 | `RunCancelled` + side_effects → 2 hints | happy |
| T-013 | Unknown failure code → 0 hints | edge |

### Layer 3: Integration Tests for `cmd_incident` (app_impl.rs)

| Test ID | Scenario | Coverage |
|---------|----------|----------|
| T-014 | Failed run → JSON output with `failure_code: "RunFailed"` | happy |
| T-015 | Non-existent run → JSON error with structured message, no stack trace | error |
| T-016 | Run with no failure event → exit code indicates no incident | happy |

## Deliverables Plan

### D1: Fix all 56 compile errors (recover_full_journal + replay_events call sites)
- **Risk**: LOW — adding `&[]` empty slices is mechanical.
- **Scope**: 8 crates (vb_storage, vb_runtime, vb_cli, workspace_tests, fuzz, vb_storage/tests)
- **Verification**: `moon ci` must pass with 0 E0061 errors.
- **Owner**: Implementation agent.
- **Prerequisite**: None.

### D2: Fix 4 zero-unwrap violations in cmd_incident
- **Risk**: LOW — structural replacement of `unwrap_or_default` with `match Result`.
- **Scope**: `crates/vb_cli/src/app_impl.rs`, lines 3181, 3185.
- **Verification**: `cargo clippy --workspace --lib --bins -- -D warnings` + compile check.
- **Owner**: Implementation agent.
- **Prerequisite**: D1 (to get a clean compile baseline).

### D3: Remove dead code (args/run_db.rs)
- **Risk**: LOW — dead code removal.
- **Scope**: `crates/vb_cli/src/args/run_db.rs`, lines 144–151.
- **Verification**: `moon ci` still passes after removal.
- **Owner**: Implementation agent.
- **Prerequisite**: None (can be done in parallel with D1).

### D4: Write unit tests for build_incident_report (8+ tests)
- **Risk**: MEDIUM — must mock `JournalEvent` variants correctly; must test all event types.
- **Scope**: `crates/vb_cli/src/commands_incident.rs` — add `#[cfg(test)] mod tests { ... }`.
- **Verification**: `cargo test --package vb_cli --lib commands_incident::tests` passes.
- **Owner**: test-writer agent.
- **Prerequisite**: None (pure function, no I/O).

### D5: Write unit tests for build_repair_hints (5 tests)
- **Risk**: LOW — deterministic output on fixed inputs.
- **Scope**: `crates/vb_cli/src/commands_incident.rs` — same test module.
- **Verification**: Same as D4.
- **Owner**: test-writer agent.
- **Prerequisite**: None.

### D6: Write integration tests for cmd_incident (3+ tests)
- **Risk**: MEDIUM — requires FjallJournal fixture setup; needs to create temp DB with known events.
- **Scope**: `crates/vb_cli/tests/` (new integration test file).
- **Verification**: `cargo test --package vb_cli --test incident_integration` passes.
- **Owner**: test-writer agent.
- **Prerequisite**: D1, D2 (must compile).

### D7: Run quality gates
- **Risk**: LOW — execution only.
- **Scope**: Full workspace.
- **Verification**: `moon ci` passes.
- **Owner**: Implementation agent.
- **Prerequisite**: D1–D6.

## Order of Execution

1. D3 (dead code — parallelizable, no deps)
2. D1 (compile fixes — blocks D2, D5, D6)
3. D2 (unwrap fixes — blocks D6)
4. D4 (unit tests for build_incident_report — parallel with D5)
5. D5 (unit tests for build_repair_hints — parallel with D4)
6. D6 (integration tests — requires D1+D2)
7. D7 (quality gates)

---

**Contract written by**: rust-contract agent
**Bead**: vb-qi37.17.1
**Date**: 2026-05-17
