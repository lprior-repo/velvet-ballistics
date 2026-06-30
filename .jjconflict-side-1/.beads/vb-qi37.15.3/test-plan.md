# Test Plan — vb-qi37.15.3: cli: Add trace command

## Summary

- **Bead:** vb-qi37.15.3
- **Feature:** CLI `trace` command — read-only journal replay showing step-by-step execution trace for a submitted run
- **Behaviors identified:** 16
- **Trophy allocation:** 9 unit / 5 integration / 1 e2e / 1 static
- **Proptest invariants:** 3
- **Fuzz targets:** 0 (run_id pre-validated by `parse_run_id` before use; Fjall is trusted storage — see TRACE-FUZZ-WAIVED)
- **Kani harnesses:** 0 (Verus covers all 18-variant determinism; TRACE-KANI-WAIVED)
- **Mutation checkpoint threshold:** ≥ 90%

---

## 1. Behavior Inventory

### Pure-Layer Behaviors (commands_journal.rs)

1. `build_trace` maps a `&[JournalEvent]` slice to a `Vec<TraceEntry>` preserving event order and count
2. `trace_one` maps each of the 18 `JournalEvent` variants to a `TraceEntry` with correct `event_type` string, `seq`, `index`, `step`, and `extra_json`
3. `build_trace` is deterministic: identical `&[JournalEvent]` always yields identical `Vec<TraceEntry>` in identical order
4. `trace_one` covers all 18 `JournalEvent` variants with no panics on any match arm
5. `TraceEntry::extra_json` is populated with variant-specific fields from each `JournalEvent`
6. For `RunResumed`, `RunRetried`, `RunAnswered` — `seq` is hardcoded to 0 (matching production)
7. `TraceEntry::step` is `Some(u16)` for step-bound events, `None` for run-level events

### I/O-Layer Behaviors (app_impl.rs)

8. `parse_run_id` accepts a valid decimal u64 string and returns `Ok(RunId)`; rejects non-numeric input with `CliExitCode::ValidationFailed`
9. `read_journal_events` returns `Err(CliExitCode::StorageError)` when the journal directory does not exist or is not readable
10. `read_journal_events` returns `Err(CliExitCode::StorageError)` when the journal read fails (corruption, I/O error)
11. `read_journal_events` returns `Ok(Vec::new())` when the run exists but has no events (not an error)
12. `cmd_trace` returns exit 0 with empty trace output when the run has no events (POST-006)
13. `cmd_trace --json` outputs a single JSON object: `{"run_id": "...", "trace": [...entries], "total": N}`
14. `cmd_trace --jsonl` outputs one JSON object per trace entry, then a final `{"total": N}` line
15. `cmd_trace` (text, default) outputs: `execution trace for run {id}`, then `  [idx] EventType step? (seq N)` lines, then `{N} event(s) total`
16. `cmd_trace` returns `CliExitCode::Success` (exit 0) on success including empty trace

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| **Unit / Calc** | 9 | `build_trace`, `trace_one` (18-variant), `parse_run_id`, `OutputFormat`, `TraceEntry` field mapping, determinism |
| **Integration** | 5 | Full `cmd_trace` against real Fjall journal; all 3 output formats; error paths (invalid db, non-existent run, journal read failure) |
| **E2E** | 1 | CLI binary invocation: `vb trace <run_id> --db <path>` black-box |
| **Static** | 1 | `cargo clippy -p vb_cli -- -D warnings` on entire vb_cli crate |

**Rationale:** Trace is I/O-heavy at the integration layer (real Fjall journal). Pure logic (`build_trace`, `trace_one`) is the widest unit-test surface. No TLA+/Kani/Loom needed (formally waived in proof-obligations.planned.jsonl). Fuzz waived — `run_id` is pre-validated by `parse_run_id` before journal lookup.

---

## 3. BDD Scenarios

### Unit: build_trace determinism

```rust
/// Behavior: build_trace produces identical output for identical input slices
/// Layer: unit
/// Test: fn build_trace_returns_identical_output_for_identical_events()
/// Inputs: slice with 3 events: RunAccepted, StepStarted, StepSucceeded
/// Assert: output[0].event_type == "RunAccepted", output[1].step == Some(step), output[2].seq matches
/// Invariant: output.len() == input.len()
```

```rust
/// Behavior: build_trace preserves event order
/// Layer: unit
/// Test: fn build_trace_preserves_event_order()
/// Assert: for all i: output[i].index == i
```

```rust
/// Behavior: build_trace on empty slice returns empty Vec
/// Layer: unit
/// Test: fn build_trace_empty_input_returns_empty_output()
/// Assert: build_trace(&[]) is empty
```

### Unit: trace_one — all 18 variants

```rust
/// Behavior: trace_one RunAccepted maps to correct TraceEntry fields
/// Layer: unit
/// Test: fn trace_one_run_accepted_sets_correct_fields()
/// Assert: event_type == "RunAccepted", step == None, seq matches, extra_json contains "run" and "workflow"
```

```rust
/// Behavior: trace_one StepStarted sets step = Some(u16)
/// Layer: unit
/// Test: fn trace_one_step_started_captures_step_number()
/// Assert: step == Some(deserialized_step), seq matches
```

```rust
/// Behavior: trace_one variants without step field set step = None
/// Layer: unit
/// Variants: RunAccepted, RunAdmission, SlotWrittenEvent, RunCancelled, RunFinished, RunFailedEvent, RunResumed, RunRetried, RunAnswered
/// Test: fn trace_one_run_level_events_have_no_step()
```

```rust
/// Behavior: trace_one RunResumed/RunRetried/RunAnswered hardcode seq = 0
/// Layer: unit
/// Test: fn trace_one_resume_retry_answer_hardcode_seq_zero()
/// Assert: seq == 0 for all three variants
```

```rust
/// Behavior: trace_one RunAnswered includes slot_idx and answer in extra_json
/// Layer: unit
/// Test: fn trace_one_run_answered_includes_extra_fields()
/// Assert: extra_json contains "slot_idx" and "answer"
```

```rust
/// Behavior: trace_one RunAdmission includes artifact_digest, granted_capabilities, policy in extra_json
/// Layer: unit
/// Test: fn trace_one_run_admission_includes_policy_fields()
/// Assert: extra_json contains "artifact_digest", "granted_capabilities", "policy"
```

### Unit: parse_run_id

```rust
/// Behavior: parse_run_id accepts valid u64 decimal string
/// Layer: unit
/// Test: fn parse_run_id_accepts_numeric_string()
/// Assert: parse_run_id("42", OutputFormat::Text) == Ok(RunId::new(42))
```

```rust
/// Behavior: parse_run_id rejects non-numeric string
/// Layer: unit
/// Test: fn parse_run_id_rejects_non_numeric_string()
/// Assert: parse_run_id("abc", OutputFormat::Text).is_err()
```

```rust
/// Behavior: parse_run_id rejects empty string
/// Layer: unit
/// Test: fn parse_run_id_rejects_empty_string()
/// Assert: parse_run_id("", OutputFormat::Text).is_err()
```

```rust
/// Behavior: parse_run_id rejects zero
/// Layer: unit
/// Test: fn parse_run_id_rejects_zero()
/// Assert: parse_run_id("0", OutputFormat::Text).is_err() — RunId::new(0) should be rejected
```

### Unit: OutputFormat

```rust
/// Behavior: OutputFormat serializes correctly for each variant
/// Layer: unit
/// Test: fn output_format_default_is_text()
/// Test: fn output_format_json_is_json()
/// Test: fn output_format_jsonl_is_jsonl()
```

### Integration: cmd_trace full pipeline

```rust
/// Behavior: trace outputs all journal events for a given run_id as ordered TraceEntry records
/// Layer: integration
/// Test: fn cmd_trace_with_events_returns_all_entries_in_order()
/// Setup: submit a workflow, collect run_id, use FjallJournal::events_for_run
/// Assert: trace entries count matches event count; indices are 0..n-1
```

```rust
/// Behavior: trace --json emits a single JSON object with run_id, trace entries array, and total
/// Layer: integration
/// Test: fn cmd_trace_json_format_structure()
/// Assert: output is valid JSON; top-level has "run_id", "trace" (array), "total" (integer)
```

```rust
/// Behavior: trace --jsonl emits one JSON object per trace entry followed by total line
/// Layer: integration
/// Test: fn cmd_trace_jsonl_format_structure()
/// Assert: last line is {"total": N}; all preceding lines are valid JSON objects
```

```rust
/// Behavior: trace on non-existent run returns exit 0 with empty trace
/// Layer: integration
/// Test: fn cmd_trace_empty_run_returns_success()
/// Assert: exit code == 0; output is empty trace or "no events found for run {id}"
```

```rust
/// Behavior: trace with invalid db path returns CliExitCode::StorageError (exit 5)
/// Layer: integration
/// Test: fn cmd_trace_invalid_db_path_returns_storage_error()
/// Assert: exit code == 5
```

### Integration: read_journal_events error paths

```rust
/// Behavior: read_journal_events returns StorageError when journal directory not found
/// Layer: integration
/// Test: fn read_journal_events_returns_storage_error_when_dir_not_found()
/// Assert: Err(CliExitCode::StorageError)
```

```rust
/// Behavior: read_journal_events returns StorageError on journal read failure
/// Layer: integration
/// Test: fn read_journal_events_returns_storage_error_on_read_failure()
/// Assert: Err(CliExitCode::StorageError)
```

### E2E: CLI binary

```rust
/// Behavior: `vb trace <run_id> --db <path>` returns exit 0 when events exist
/// Layer: e2e
/// Test: fn cli_trace_command_exit_code_success()
/// Command: process::Command::new("vb").arg("trace").arg("1").arg("--db").arg(db_path)
/// Assert: exit code == 0
```

### Error variant scenarios

```rust
/// Behavior: trace with invalid db path returns non-zero exit
/// Given: db path does not exist
/// When: vb trace 1 --db /nonexistent/path
/// Then: exit code != 0
```

```rust
/// Behavior: trace with invalid run_id format returns ValidationFailed exit
/// Given: run_id is "not-a-number"
/// When: vb trace not-a-number --db /path
/// Then: exit code == 1 (ValidationFailed)
```

---

## 4. Proptest Invariants

### Proptest: build_trace determinism

```
Invariant: ∀ events1, events2: if events1 == events2 then build_trace(events1) == build_trace(events2)
Strategy: Vec<JournalEvent> — generate vectors of 0..20 events, each event one of the 18 variants with valid field values
Anti-invariant: different input slices should produce different outputs
```

### Proptest: build_trace completeness

```
Invariant: ∀ events: build_trace(events).len() == events.len()
Strategy: generate any non-empty Vec<JournalEvent> and verify output length matches input length
```

### Proptest: trace_one index correspondence

```
Invariant: ∀ idx, event: trace_one(idx, &event).index == idx
Strategy: generate arbitrary idx (0..1000) and arbitrary JournalEvent; verify the returned TraceEntry.index matches
```

---

## 5. Fuzz Targets

**Waived.** `run_id` is pre-validated by `parse_run_id` before journal lookup (TRACE-FUZZ-WAIVED). Fjall journal is trusted storage. No raw bytes enter the trace pipeline.

---

## 6. Kani Harnesses

**Waived.** TRACE-KANI-WAIVED: Verus exhaustive match proof (`proof_trace_one_variant_coverage`) covers all 18 variants. Kani would add CI cost without catching distinct defects. Formal determinism covered by `proof_trace_one_applied_globally_deterministic`.

---

## 7. Mutation Checkpoints

Critical mutations that **must** be caught:

| Function | Mutation | Catch by test |
|---|---|---|
| `trace_one` match | Remove any match arm | `trace_one_variant_coverage` (unit) |
| `build_trace` | `enumerate().map(trace_one)` → skip index | `build_trace_preserves_event_order` |
| `build_trace` | `collect()` → reverse | `build_trace_preserves_event_order` |
| `trace_one` | Hardcode `seq: 0` for RunResumed (already 0) | Explicit `seq == original_seq.get()` check |
| `parse_run_id` | Accept zero as valid | `parse_run_id_rejects_zero` |
| `parse_run_id` | Accept non-numeric | `parse_run_id_rejects_non_numeric_string` |
| `cmd_trace` | Return success on storage error | `cmd_trace_invalid_db_path_returns_storage_error` |
| `trace_entry_to_json` | Omit `step` field when None | `trace_entry_to_json_omits_none_step` |

**Threshold:** ≥ 90% mutation kill rate via `cargo mutants`.

---

## 8. Combinatorial Coverage Matrix

### Unit: build_trace / trace_one

| Scenario | Input | Expected Output | Layer |
|---|---|---|---|
| build_trace empty | `&[]` | empty `Vec` | unit |
| build_trace 1 event | one RunAccepted | `Vec` with 1 entry, index=0 | unit |
| build_trace 3 events | RunAccepted, StepStarted, StepSucceeded | 3 entries, indices 0,1,2 | unit |
| trace_one each variant | all 18 `JournalEvent` variants | `TraceEntry` with correct `event_type` | unit |
| trace_one step events | StepStarted | `step: Some(u16)`, no extra_json step key | unit |
| trace_one non-step events | RunAccepted | `step: None` | unit |
| trace_one RunResumed | RunResumed | `seq: 0` | unit |
| trace_one RunRetried | RunRetried | `seq: 0` | unit |
| trace_one RunAnswered | RunAnswered | includes `slot_idx`, `answer` in extra_json | unit |
| trace_one determinism | same (idx, event) twice | identical `TraceEntry` | unit/proptest |

### Unit: parse_run_id

| Scenario | Input | Expected Output | Layer |
|---|---|---|---|
| valid u64 | "123" | `Ok(RunId::new(123))` | unit |
| zero | "0" | `Err(CliExitCode::ValidationFailed)` | unit |
| non-numeric | "abc" | `Err(CliExitCode::ValidationFailed)` | unit |
| empty | "" | `Err(CliExitCode::ValidationFailed)` | unit |
| negative | "-1" | `Err(CliExitCode::ValidationFailed)` | unit |

### Integration: cmd_trace with real journal

| Scenario | Input | Expected Output | Layer |
|---|---|---|---|
| trace with events (text) | real Fjall journal with ≥3 events | exit 0, ordered text lines with indices | integration |
| trace with events (--json) | same journal | valid JSON object with run_id, trace array, total | integration |
| trace with events (--jsonl) | same journal | N entry lines + final total line | integration |
| trace empty run | non-existent run_id | exit 0, empty trace | integration |
| trace invalid db path | nonexistent path | exit 5, StorageError | integration |
| trace journal read error | corrupted journal | exit 5, StorageError | integration |

---

## Open Questions

- **Q1:** Is `RunId::new(0)` actually rejected, or does it produce a valid zero-id run? The `parse_run_id` in `app_impl.rs` uses `raw.parse::<u64>()` then `RunId::new(id)`. The rejection of zero depends on `RunId::new`'s internal validation. Unit test `parse_run_id_rejects_zero` will confirm.
- **Q2:** Is there a `JournalError` variant that maps to something other than `StorageError`? The `From<JournalError>` impl in `exit_code.rs` maps all variants to `StorageError`. Integration test for corrupted journal read should confirm this.
- **Q3:** Does `--db` path validation happen before or after `parse_run_id`? From code inspection, `parse_run_id` is called first; if it fails, storage is never opened. Integration test with valid run_id but invalid db path will confirm error ordering.

---

## Test Location

| Test Group | Location |
|---|---|
| Unit: commands_journal | `crates/vb_cli/src/commands_journal.rs` (inline `#[cfg(test)]`) |
| Unit: parse_run_id | `crates/vb_cli/src/app_impl.rs` (inline `#[cfg(test)]`) |
| Unit: OutputFormat | `crates/vb_cli/src/args.rs` (existing tests) |
| Integration: cmd_trace | `crates/vb_cli/tests/cli_integration.rs` (new section for `mod trace`) |
| Proptest: build_trace | `crates/vb_cli/src/commands_journal.rs` or `crates/workspace_tests/tests/` |
| E2E: CLI binary | `crates/workspace_tests/tests/` or `crates/vb_cli/tests/cli_integration.rs` |
