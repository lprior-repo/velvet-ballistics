# Test Plan: vb-qi37.16.2 — Durable Resume Transition

## Summary

| Metric | Value |
|--------|-------|
| Bead ID | vb-qi37.16.2 |
| Feature | cli/runtime: Implement durable resume transition |
| Behaviors identified | 11 contract clauses |
| Trophy allocation | ~30% unit / ~60% integration / ~5% e2e / ~5% static |
| Proptest invariants | 5 |
| Fuzz targets | 2 |
| Kani harnesses | 2 |
| Mutation checkpoints | 6 |
| Release critical | true |
| Risk tags | p0, durability, journal-replay, cli-runtime-boundary, state-transition |

---

## 1. Behavior Inventory

### Preconditions (PRE)

| ID | Behavior |
|----|----------|
| PRE-001 | **Journal run_id existence**: The caller must provide a `run_id` that exists in the runtime journal |
| PRE-002 | **Resumable state requirement**: The runtime state for the given `run_id` must be in a `Resumable` variant (not `Initial`, not `Running`, not `Failed`) |
| PRE-003 | **Hydration completeness**: Journal hydration for `run_id` must be complete (all prior events are present and reconstructable) |

### Postconditions (POST)

| ID | Behavior |
|----|----------|
| POST-001 | **Journal append before success**: On successful resume, runtime transitions `Resumable -> Running` and `RuntimeJournalEvent::Resumed` is appended to the journal **before** success is returned |
| POST-002 | **Structured result output**: On successful resume, structured output is produced containing `run_id`, `status="resumed"`, and `timestamp` |
| POST-003 | **Fail-closed error handling**: On failed resume (due to PRE violation), the runtime remains in the original state and an appropriate `Error` variant is returned |
| POST-004 | **Durable journal evidence**: `RuntimeJournalEvent::Resumed` is append-only and durable before success is reported to the caller |

### Invariants (INV)

| ID | Behavior |
|----|----------|
| INV-001 | **Valid state machine transitions**: The runtime state machine never transitions to `Running` except via a valid `Resume` transition from `Resumable` |
| INV-002 | **Journal immutability**: Journal events are never reordered, deleted, or modified after append |
| INV-003 | **ResumeResult field presence**: `ResumeResult` output always contains `run_id`, `status`, and `timestamp` fields |
| INV-004 | **Failed-not-resumable**: A `run_id` in `Failed` state is not resumable (resume from `Failed` returns `Error::NotResumable`) |

---

## 2. Trophy Allocation

### Layer Breakdown

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Static Analysis** | 3 checks | `cargo clippy`, `cargo-deny` license audit, compile-fail test for `JournalError` typo fix (contract-verification-review.md:55) |
| **Unit / Calc** | 25 tests | Pure lifecycle transition logic, `RuntimeState::is_resumable` predicate, journal append-only SEQ behavior, `ResumeResult` field presence, error propagation paths, AlreadyRunning variant handling, StructuredOutputFailed schema validation |
| **Integration** | 9 tests | CLI-runtime boundary routing, journal replay survival, incomplete hydration rejection, journal unchanged after replay, output format validation |
| **E2E** | 2 tests | Full `velvet_ballistics resume` CLI command end-to-end with temp database |
| **Property-based** | 5 invariants | RuntimeState exhaustive variant coverage, is_resumable predicate, is_hydration_complete, append immutability, result field presence |
| **Formal (TLA+)** | 4 model-checks | `ValidTransition`, `JournalAppendBeforeSuccess`, `JournalImmutable`, `FailedNotResumable` invariants |
| **Formal (Verus)** | 5 proofs | Pure state predicates, append-only SEQ, typestate field presence |

**Total: 53 tests + 4 TLA+ checks + 5 Verus proofs**

### Target Ratios vs Actual

| Target | Actual |
|--------|--------|
| ~5% static | 6% (3 checks) |
| ~30% unit | 47% (25 tests) |
| ~60% integration | 38% (9 tests) + 13% formal |
| ~5% e2e | 4% (2 tests) |

Deviation from pure trophy: higher formal layer weight (TLA+ + Verus) is justified by `release_critical=true` and `risk_tag=p0 durability`.

---

## 3. BDD Scenarios

### PRE-001: run_id must exist in journal

#### Scenario: `cli_resume_run_id_not_found`
```
Given: a runtime journal with no entry for run_id "run-999"
When:  velvet_ballistics resume is invoked with run_id="run-999"
Then:  Command::Resume returns Error::RunIdNotFound("run-999")
And:   runtime state is unchanged (no transition attempted)
```

#### Scenario: `cli_resume_run_id_exists`
```
Given: a runtime journal with an existing entry for run_id "run-001" in Resumable state
When:  velvet_ballistics resume is invoked with run_id="run-001"
Then:  resume proceeds to Running state
And:   RuntimeJournalEvent::Resumed is appended
```

---

### PRE-002: runtime state must be Resumable

#### Scenario: `lifecycle_resume_from_resumable_succeeds`
```
Given: RuntimeState[run_id] = Resumable
When:  Shard::handle_resume(run_id) is called
Then:  Result::Ok(ResumeResult { run_id, status: ResumeStatus::Resumed, timestamp })
And:   RuntimeState transitions to Running
```

#### Scenario: `lifecycle_resume_from_initial_fails`
```
Given: RuntimeState[run_id] = Initial
When:  Shard::handle_resume(run_id) is called
Then:  Result::Err(ResumeError::NotResumable { run_id, current_state: Initial })
And:   RuntimeState remains Initial
```

#### Scenario: `lifecycle_resume_from_running_fails`
```
Given: RuntimeState[run_id] = Running
When:  Shard::handle_resume(run_id) is called
Then:  Result::Err(ResumeError::NotResumable { run_id, current_state: Running })
And:   RuntimeState remains Running
```

#### Scenario: `lifecycle_resume_from_resuming_fails`
```
Given: RuntimeState[run_id] = Resuming (another resume in-flight)
When:  Shard::handle_resume(run_id) is called
Then:  Result::Err(ResumeError::NotResumable { run_id, current_state: Resuming })
And:   RuntimeState remains Resuming
```

#### Scenario: `lifecycle_resume_from_already_running_returns_already_running`
```
Given: RuntimeState[run_id] = Running (run is actively executing)
When:  Shard::handle_resume(run_id) is called
Then:  Result::Ok(ResumeResult { run_id, status: AlreadyRunning, timestamp })
And:   RuntimeState remains Running (no state change)
And:   ResumeResult.status == ResumeStatus::AlreadyRunning exactly
And:   Journal is NOT appended (idempotent — no duplicate Resumed events)
And:   exit code is 0 (not an error path, but AlreadyRunning is a success variant)
```

#### Scenario: `lifecycle_resume_from_failed_fails`
```
Given: RuntimeState[run_id] = Failed
When:  Shard::handle_resume(run_id) is called
Then:  Result::Err(ResumeError::NotResumable { run_id, current_state: Failed })
And:   RuntimeState remains Failed
```

---

### PRE-003: journal hydration must be complete

#### Scenario: `resume_incomplete_hydration_fails`
```
Given: a journal with gap in event sequence for run_id "run-002" (missing intermediate event)
When:  Shard::handle_resume("run-002") is called
Then:  Result::Err(ResumeError::IncompleteHydration("run-002"))
And:   RuntimeState remains unchanged
```

#### Scenario: `resume_complete_hydration_succeeds`
```
Given: a complete journal event sequence for run_id "run-003" (all prior events present)
When:  Shard::handle_resume("run-003") is called
Then:  resume proceeds successfully
```

---

### POST-001: journal append before success

#### Scenario: `resume_appends_journal_before_success`
```
Given: RuntimeState[run_id] = Resumable and journal is at length N
When:  Shard::handle_resume(run_id) is called and returns Ok
Then:  Journal[N] = RuntimeJournalEvent::Resumed { run_id, timestamp }
And:   Journal length is N+1
And:   RuntimeState = Running
And:   the Resumed event was appended BEFORE success is returned (enforced by PendingResume tracking)
```

#### Scenario: `journal_append_failure_returns_error`
```
Given: RuntimeState[run_id] = Resumable but journal append would fail (simulated disk full)
When:  Shard::handle_resume(run_id) is called
Then:  Result::Err(ResumeError::JournalAppendFailed)
And:   RuntimeState remains Resumable (no partial state)
```

---

### POST-002: structured result output

#### Scenario: `cli_resume_output_format`
```
Given: a valid resumable run_id "run-004"
When:  velvet_ballistics resume --run-id run-004 --db /tmp/test.db --output json is invoked
Then:  stdout contains valid JSON where run_id == "run-004" exactly
And:   status == "resumed" exactly (not "running", not "already_running")
And:   timestamp conforms to ISO 8601 format
And:   exit code is 0
And:   no error fields are present in the JSON output
```

#### Scenario: `cli_resume_output_format_yaml`
```
Given: a valid resumable run_id "run-005"
When:  velvet_ballistics resume --run-id run-005 --db /tmp/test.db --output yaml is invoked
Then:  stdout contains valid YAML with keys: run_id, status, timestamp
And:   exit code is 0
```

#### Scenario: `structured_output_failure_returns_partial_with_error`
```
Given: a valid resumable run_id "run-006" but output formatting fails
When:  velvet_ballistics resume is invoked
Then:  ResumeResult is still produced internally
And:   Error::StructuredOutputFailed is logged but does not block success
And:   exit code is 0 (non-fatal error)
```

#### Scenario: `structured_output_failed_result_schema`
```
Given: a valid resumable run_id "run-006" and output formatting is configured but fails
When:  velvet_ballistics resume is invoked
Then:  Error::StructuredOutputFailed result schema is:
│ RunId:     Some("run-006")          // run_id preserved in error context
│ ErrorKind: StructuredOutputFailed   // exact error variant
│ Inner:     Some(FormatError)        // underlying format error (json/yaml/...)
│ Timestamp: Some(UtcDateTime)        // timestamp of the failure event
│ Message:   Some(String)             // human-readable format error description
And:  the error contains run_id == "run-006" (error is traceable to operation)
And:  the error contains ErrorKind == StructuredOutputFailed (exact variant match)
And:  the error contains Inner error with FormatError variant
And:  ResumeResult internal state reflects Running (journal was appended despite output failure)
And:  Error::StructuredOutputFailed is logged to structured log at ERROR level
```

---

### POST-003: fail-closed on invalid resume

#### Scenario: `lifecycle_resume_error_propagation`
```
Given: RuntimeState[run_id] = Initial
When:  Shard::handle_resume(run_id) is called
Then:  Err(ResumeError::NotResumable) is returned
And:   RuntimeState is unchanged (still Initial)
And:   no journal event is appended
```

#### Scenario: `resume_not_resumable_error`
```
Given: RuntimeState[run_id] = Running
When:  Shard::handle_resume(run_id) is called
Then:  Err(ResumeError::NotResumable { run_id, current_state: Running }) is returned
And:   RuntimeState is unchanged
```

---

### POST-004: durable journal evidence

#### Scenario: `replay_resume_journal_unchanged`
```
Given: a journal with Resumed event for run_id "run-007"
When:  the runtime is restarted and journal is replayed
Then:  the Resumed event is replayed identically
And:   RuntimeState[run_id] = Running after replay
And:   no additional Resumed events are appended (idempotent replay)
```

---

### INV-001: valid state machine transitions

#### Scenario: `lifecycle_state_machine_invariants`
```
Given: all possible RuntimeState variants: Initial, Running, Resumable, Resuming, Failed
When:  for each variant we attempt handle_resume
Then:  only Resumable variant permits resume
And:   all other variants return NotResumable
And:   no invalid transitions occur (e.g., Initial->Running without Resume)
And:   Invariant: Running is reached only via Resumable->Resuming->Running
```

---

### INV-002: journal append-only

#### Scenario: `journal_append_only`
```
Given: a RuntimeJournal with existing events
When:  RuntimeJournal::append(Resumed { run_id, timestamp }) is called
Then:  the event is added to the end of the journal SEQ
And:   no existing events are modified or reordered
And:   no events are deleted
And:   subsequent get_state(run_id) reflects the new event
```

#### Scenario: `journal_append_immutable_under_concurrent_access`
```
Given: a RuntimeJournal with events for multiple run_ids
When:  concurrent appends occur for different run_ids
Then:  all events are recorded in append order
And:   no event is lost or overwritten
And:   journal SEQ length equals number of appends
```

---

### INV-003: ResumeResult field presence

#### Scenario: `resume_result_field_presence`
```
Given: a successful resume of run_id "run-008"
When:  ResumeResult is returned
Then:  ResumeResult.run_id == "run-008"
And:   ResumeResult.status == ResumeStatus::Resumed (or AlreadyRunning if already running)
And:   ResumeResult.timestamp is a valid UtcDateTime
And:   all three fields are non-None
```

---

### INV-004: Failed not resumable

#### Scenario: `resume_failed_run_id_error`
```
Given: RuntimeState["run-009"] = Failed { reason: "panic in task" }
When:  Shard::handle_resume("run-009") is called
Then:  Result::Err(ResumeError::NotResumable { run_id: "run-009", current_state: Failed })
And:   RuntimeState["run-009"] remains Failed
And:   Invariant FailedNotResumable is preserved
```

---

## 4. Proptest Invariants

### Proptest: `RuntimeState::is_resumable`
- **Invariant**: `is_resumable` returns `true` **only** for the `Resumable` variant and `false` for all other variants (`Initial`, `Running`, `Resuming`, `Failed`)
- **Strategy**: `proptest::prop_oneof![Just(RuntimeState::Initial), Just(RuntimeState::Running), Just(RuntimeState::Resumable), Just(RuntimeState::Resuming), Just(RuntimeState::Failed)]`
- **Anti-invariant**: any call where `is_resumable` returns `true` for a non-`Resumable` variant must be caught

### Proptest: `RuntimeJournal::is_hydration_complete`
- **Invariant**: `is_hydration_complete(run_id)` returns `true` only when all events in the sequence for `run_id` are present and in order
- **Strategy**: generate valid event sequences, then corrupt them (remove middle event, swap order, duplicate) and verify `is_hydration_complete` returns `false`

### Proptest: `RuntimeJournal::append` immutability
- **Invariant**: after `append(event)`, the original events at indices 0..N are unchanged
- **Strategy**: generate a journal of length N, append an event, verify original events at 0..N-1 are bit-for-bit identical

### Proptest: `Shard::handle_resume` state transition validity
- **Invariant**: after `handle_resume` returns `Ok`, `RuntimeState[run_id] == Running` and the previous state was `Resumable`
- **Strategy**: generate valid `Resumable` states, verify transition produces `Running`

### Proptest: `ResumeResult` field completeness
- **Invariant**: for any successful `ResumeResult`, all three fields (`run_id`, `status`, `timestamp`) are populated with non-empty/non-zero values
- **Strategy**: generate 1000 successful resume operations and verify each result has all fields populated

---

## 5. Fuzz Targets

### Fuzz Target: `RuntimeJournalEvent::Resumed` deserialization
- **Input type**: arbitrary bytes / malformed event data
- **Risk**: panic on deserialization, OOM on oversized fields, logic error on out-of-order fields
- **Corpus seeds**: valid `Resumed { run_id: "run-001", timestamp: 2026-01-01T00:00:00Z }`, empty run_id, unicode in run_id, negative timestamp, timestamp year out of range
- **Target function**: `RuntimeJournalEvent::try_from` or equivalent deserializer
- **Expected**: no panic; return `Err` for malformed input

### Fuzz Target: `Command::Resume` CLI argument parsing
- **Input type**: arbitrary string arguments to `velvet_ballistics resume`
- **Risk**: argument parser panic, path traversal in `--db`, format string injection in `--output`
- **Corpus seeds**: valid args, empty run_id, run_id with special chars, non-existent db path, invalid output format
- **Target function**: `args::parse_resume` or equivalent
- **Expected**: graceful error message; no panic

---

## 6. Kani Harnesses

### Kani Harness: `Shard::handle_resume` state machine invariant
- **Property**: For all possible `RuntimeState` variants and all valid `run_id` inputs, `handle_resume` either:
  1. Returns `Ok(ResumeResult)` with `RuntimeState` = `Running` AND previous state was `Resumable`, OR
  2. Returns `Err(ResumeError)` AND `RuntimeState` is unchanged
- **Bound**: 5-state enum × 10 run_id values × 2^4 journal event flags
- **Rationale**: formal proof that no invalid state transitions occur; critical for p0 durability risk

### Kani Harness: `RuntimeJournal::append` ordering guarantee
- **Property**: For any journal `J` and any event `e`, after `J.append(e)`, `J[i]` for all `i < len(J)-1` are bit-for-bit identical to the original values before append
- **Bound**: journal length up to 100, 4 event types × 10 run_ids
- **Rationale**: formal proof that append is truly append-only; no out-of-bounds write, no corruption of existing entries

---

## 7. Mutation Checkpoints

| Mutation | Target | Catch by test |
|---------|--------|---------------|
| Change `Resumable` guard to `true` unconditionally | `Shard::handle_resume` | `lifecycle_resume_from_running_fails`, `lifecycle_resume_from_initial_fails` |
| Remove `Journal::append` call before success return | POST-001 path | `resume_appends_journal_before_success` |
| Remove `is_hydration_complete` check | PRE-003 path | `resume_incomplete_hydration_fails` |
| Change `NotResumable` error variant to `RunIdNotFound` | error taxonomy | `lifecycle_resume_from_failed_fails` (asserts NotResumable variant) |
| Remove `is_resumable` predicate check | PRE-002 path | `lifecycle_resume_from_initial_fails`, `lifecycle_resume_from_running_fails` |
| Flip append-only SEQ to mutable vector | `RuntimeJournal::append` | `journal_append_only` (verifies immutability) |

**Threshold**: ≥90% mutation kill rate (enforced by `cargo-mutants --score-threshold 90`)

---

## 8. Combinatorial Coverage Matrix

### Unit: Lifecycle Transition Tests

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|-----------------|------------|
| resume from Initial | `RuntimeState::Initial` | `Err(NotResumable)` | unit |
| resume from Running | `RuntimeState::Running` | `Ok(ResumeResult { status: AlreadyRunning })` | unit |
| resume from Resumable (happy) | `RuntimeState::Resumable` | `Ok(ResumeResult { Running })` | unit |
| resume from Resuming | `RuntimeState::Resuming` | `Err(NotResumable)` | unit |
| resume from Failed | `RuntimeState::Failed` | `Err(NotResumable)` | unit |
| journal append failure | `append()` returns Err | `Err(JournalAppendFailed)` | unit |
| error preserves state: Initial | PRE violation from Initial | state unchanged | unit |
| error preserves state: Running | PRE violation from Running | state unchanged | unit |
| error preserves state: Resuming | PRE violation from Resuming | state unchanged | unit |
| AlreadyRunning result schema | Running state resume | `Ok(ResumeResult { run_id, status: AlreadyRunning, timestamp })` | unit |
| AlreadyRunning journal idempotency | Running state resume | no journal append | unit |

### Unit: Journal Append-Only Tests

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|-----------------|------------|
| append single event | empty journal | event at index 0 | unit |
| append preserves index 0 | journal[0]=E1, append E2 | journal[0]==E1, journal[1]==E2 | unit |
| append increases length by 1 | journal len=N | append => len=N+1 | unit |
| get_state reflects new event | append Resumed | get_state returns updated state | unit |
| concurrent appends (serial) | multiple appends | all events present in order | unit |

### Unit: ResumeResult Field Presence Tests

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|-----------------|------------|
| successful resume has all fields | Ok result | run_id != None, status != None, timestamp != None | unit |
| error result has run_id | Err result | run_id field populated | unit |
| status is Resumed variant | Ok result | status == ResumeStatus::Resumed | unit |
| status is AlreadyRunning variant | Ok result (already running) | status == ResumeStatus::AlreadyRunning | unit |
| AlreadyRunning result has all fields | Ok result | run_id != None, status != None, timestamp != None | unit |
| AlreadyRunning status is not error | Ok result | result is Ok (AlreadyRunning is success variant) | unit |

### Integration: CLI-Runtime Boundary Tests

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|-----------------|------------|
| valid resume routing | `Command::Resume { run-001 }` | Ok + output | integration |
| run_id not found routing | `Command::Resume { run-999 }` | Err(RunIdNotFound) | integration |
| structured output: JSON | `--output json` | valid JSON with fields | integration |
| structured output: YAML | `--output yaml` | valid YAML with fields | integration |
| incomplete hydration | gap in journal | Err(IncompleteHydration) | integration |
| journal unchanged after replay | existing journal | replay produces identical state | integration |

### Integration: Journal Replay Tests

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|-----------------|------------|
| replay resumes to Running | journal with Resumed | state == Running | replay |
| replay is idempotent | replay twice | same state, no duplicate events | replay |
| replay with missing event | incomplete journal | IncompleteHydration error | replay |
| journal unchanged after replay | replay | original journal == replayed journal | replay |

### E2E: Full CLI Invocation Tests

| Scenario | Input | Expected Output | Test Layer |
|----------|-------|-----------------|------------|
| `velvet_ballistics resume --run-id X` | valid resumable run | exit 0 + structured output | e2e |
| `velvet_ballistics resume --run-id X` | non-existent run | exit != 0 + error message | e2e |

---

## 9. Error Variant Coverage

All 5 error variants in `ResumeError` enum must have explicit test scenarios:

| Error Variant | Traced to Clause | Test Scenarios |
|--------------|------------------|----------------|
| `RunIdNotFound(RunId)` | PRE-001 | `cli_resume_run_id_not_found` |
| `NotResumable { run_id, current_state }` | PRE-002, INV-004 | `lifecycle_resume_from_initial_fails`, `lifecycle_resume_from_running_fails`, `lifecycle_resume_from_failed_fails`, `resume_failed_run_id_error` |
| `IncompleteHydration(RunId)` | PRE-003 | `resume_incomplete_hydration_fails` |
| `JournalAppendFailed` | POST-004 | `journal_append_failure_returns_error` |
| `StructuredOutputFailed` | POST-002 | `structured_output_failure_returns_partial_with_error`, `structured_output_failed_result_schema` |

**Note**: `AlreadyRunning` is a `ResumeStatus` success variant, not an error. It is tested via `lifecycle_resume_from_already_running_returns_already_running`.

**Note**: Contract typo at contract.md:79 — `JurnalError` should be `JournalError`. Test names should use the corrected spelling.

---

## 10. Verification Traceability

### Test-to-Proof Obligation Map

| Test Scenario | Proof Obligation(s) |
|---------------|-------------------|
| `cli_resume_run_id_not_found` | VERUS-PRE-003 |
| `lifecycle_resume_from_resumable_succeeds` | VERUS-PRE-002, TLA-RESUME-004 |
| `resume_incomplete_hydration_fails` | VERUS-PRE-003, TLA-RESUME-003 |
| `resume_appends_journal_before_success` | TLA-RESUME-001, TLA-RESUME-002, VERUS-POST-004, INTEGRATION-REPLAY-001 |
| `cli_resume_output_format` | INTEGRATION-CLI-001 |
| `lifecycle_resume_error_propagation` | TLA-RESUME-003 |
| `replay_resume_journal_unchanged` | VERUS-POST-004, TLA-RESUME-002, INTEGRATION-REPLAY-001 |
| `lifecycle_state_machine_invariants` | TLA-RESUME-001, VERUS-INV-001, UNIT-LIFECYCLE-001 |
| `journal_append_only` | VERUS-POST-004, TLA-RESUME-001 |
| `resume_result_field_presence` | VERUS-INV-003 |
| `lifecycle_resume_from_failed_fails` | TLA-RESUME-004, VERUS-PRE-002 |

### Formal Verification Commands (from proof-obligations.jsonl)

```bash
# TLA+ model checking
tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla

# Verus proofs
verus crates/vb_runtime/src/shard/lifecycle.rs crates/vb_runtime/src/shard/types.rs
verus crates/vb_runtime/src/journal.rs

# Integration
cargo test --package vb_storage --test replay_resume -- --nocapture
cargo test --package velvet_ballistics --test cli_integration resume -- --nocapture

# Unit
cargo test --package vb_runtime --lib -- shard::lifecycle::tests -- --nocapture

# Proptest
cargo test --package vb_runtime --lib -- properties -- --nocapture
```

---

## 11. Open Questions

1. **StructuredOutputFailed non-fatal scope**: POST-002 states the error is "non-fatal, returns partial result with error tag". Is partial result defined as `ResumeResult` with an error tag embedded, or a completely separate output format? Awaiting clarification on `ResumeResult` schema for error-tagged outputs.

2. **Idempotency boundary for AlreadyRunning**: `ResumeStatus` has `AlreadyRunning` variant. Is a second resume of an already-running run_id considered a no-op (returns `Ok(ResumeResult { status: AlreadyRunning })`) or an error? TLA+ refinement suggests `ResumeIdempotent` invariant but test behavior is not explicit.

3. **Journal replay storage backend**: INTEGRATION-REPLAY-001 targets `vb_storage` package. Is a real FJALL backend used or an in-memory fake? Contract says storage backend excluded from formal proof, but integration tests should use real backend for durability confidence.

4. **Concurrency model for journal append**: INV-002 (append-only) is proven for sequential access. Is concurrent append from multiple run_ids supported in the same runtime instance? TLA+ `PendingResume` and `ResumedSet` suggest single-run_id focus, but `JournalImmutable` may need clarification.

5. **Contract typo JurnalError**: Minor typo at contract.md:79 must be fixed before test naming finalizes. Use `JournalError` consistently.

---

## 12. Test File Locations

| Test Type | Location |
|-----------|----------|
| Unit (lifecycle) | `crates/vb_runtime/src/shard/lifecycle.rs` (inline `#[cfg(test)]` module) |
| Unit (journal) | `crates/vb_runtime/src/journal.rs` (inline `#[cfg(test)]` module) |
| Unit (types) | `crates/vb_runtime/src/shard/types.rs` (inline `#[cfg(test)]` module) |
| Integration (CLI) | `crates/velvet_ballistics/tests/cli_integration.rs` |
| Integration (replay) | `crates/vb_storage/tests/replay_resume.rs` |
| Property-based | `crates/vb_runtime/src/shard/properties.rs` or `crates/vb_runtime/src/properties.rs` |
| E2E | `crates/velvet_ballistics/tests/e2e_resume.rs` |
| Fuzz | `fuzz/fuzz_targets/resume_deserialization.rs` |

---

*Test plan produced from contract artifacts: `contract.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `contract-verification-review.md` (approved 2026-05-11).*
