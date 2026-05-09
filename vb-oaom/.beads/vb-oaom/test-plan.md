# Test Plan — vb-oaom: cli: Add runtime ai context packet command

## Section 1 — Behavior Inventory

### `parse_run_id`
- `parse_run_id returns RunId when input is valid decimal u64`
- `parse_run_id returns ValidationFailed when input is non-numeric`
- `parse_run_id returns ValidationFailed when input is out-of-range decimal`
- `parse_run_id returns ValidationFailed when input is empty string`
- `parse_run_id returns ValidationFailed when input has whitespace prefix`

### `handle`
- `handle emits AiContextPacket JSON when run exists with events`
- `handle emits [REDACTED] for secret-tainted slot values in output`
- `handle resolves workflow digest to available compiled IR`
- `handle infers action IDs from both journal events and compiled IR`
- `handle returns ValidationFailed with RUN_NOT_FOUND when run has zero events`
- `handle returns StorageError when journal cannot be opened`
- `handle returns StorageError when events_for_run fails`
- `handle returns StorageError when run_header fails`
- `handle returns StorageError when latest_snapshot fails`
- `handle returns StorageError when journal file is corrupt`
- `handle degrades gracefully when latest_snapshot returns None`

### `redacted_slot_value`
- `redacted_slot_value returns [REDACTED] when slot is Secret (taint=1)`
- `redacted_slot_value returns [REDACTED] when slot is DerivedFromSecret (taint=2)`
- `redacted_slot_value returns decoded SlotValue string when slot is Clean`
- `redacted_slot_value returns [UNDECODED] when bytes fail postcard decode`
- `redacted_slot_value returns Null when value is None and slot is Clean`
- `redacted_slot_value returns Null when taint=0 and value is empty Vec<u8>`

### `slot_is_secret_or_derived`
- `slot_is_secret_or_derived returns true when taint entry is 1`
- `slot_is_secret_or_derived returns true when taint entry is 2`
- `slot_is_secret_or_derived returns false when taint entry is 0`
- `slot_is_secret_or_derived returns false when snapshot is None`
- `slot_is_secret_or_derived returns false when slot index beyond taint table`

### `suggested_ai_commands`
- `suggested_ai_commands returns inspect and events for all statuses`
- `suggested_ai_commands adds incident and retry when status is Failed`
- `suggested_ai_commands adds incident and retry when status is Cancelled`
- `suggested_ai_commands adds trace and resume when status is Running`
- `suggested_ai_commands adds replay when status is Finished`
- `suggested_ai_commands returns max 4 commands`
- `suggested_ai_commands all commands start with velvet-ballastics`

### `ai_workflow_summary`
- `ai_workflow_summary returns null digest when no digest available`
- `ai_workflow_summary returns compiled_ir available false when IR not found`
- `ai_workflow_summary returns compiled_ir available true with full node data`
- `ai_workflow_summary returns referenced_actions from Do nodes`
- `ai_workflow_summary returns compiled_ir available false when IR decode fails`

### `ai_action_contracts`
- `ai_action_contracts returns unique action IDs from events`
- `ai_action_contracts returns action IDs from workflow_actions`
- `ai_action_contracts each entry has contract_status inferred_from_compiled_ir_and_journal`

### `report_run_not_found`
- `report_run_not_found outputs JSON with code RUN_NOT_FOUND`
- `report_run_not_found returns CliExitCode::ValidationFailed`

### `run_status_from_events`
- `run_status_from_events returns Finished when last event is RunFinished`
- `run_status_from_events returns Failed when last event is RunFailedEvent`
- `run_status_from_events returns Cancelled when last event is RunCancelled`
- `run_status_from_events returns Running otherwise`
- `run_status_from_events returns Running when events list is empty`

---

## Section 2 — Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| Unit #[cfg(test)] | 20 | Pure calc-layer functions: `parse_run_id`, `redacted_slot_value`, `slot_is_secret_or_derived`, `suggested_ai_commands`, `ai_workflow_summary`, `ai_action_contracts`, `run_status_from_events`, `report_run_not_found`, `latest_snapshot_from_events` |
| Integration /tests/ | 8 | Full `handle` CLI path with real Fjall journal covering: valid run, secret redaction, IR resolution, action inference, zero events, corrupt journal, snapshot degradation, run_header failure |
| Proptest | 3 | `redacted_slot_value` (secret/derived always redact, clean never), `parse_run_id` (arbitrary strings), `suggested_ai_commands` (bounded length, prefix) |
| Cargo-Fuzz | 1 | `redacted_slot_value` against malformed `Vec<u8>` payloads — INV-004 |
| Kani | 1 | `redacted_slot_value` — no panic on arbitrary `SlotIdx` + `Option<Vec<u8>>` — POST-003-KANI |
| Static analysis | 3 | Clippy, machete, no-unwrap enforcement |
| Manual QA | 4 | Live CLI invocation: valid run, invalid run_id, nonexistent db path, run with secret slots |

**Target ratio**: ~60% integration, ~30% unit, ~5% e2e/manual, ~5% static/proptest/fuzz/kani

---

## Section 3 — BDD Scenarios

### Behavior: `parse_run_id returns RunId when input is valid decimal u64`
Given: no preconditions (pure function)
When: `parse_run_id("12345")`
Then: returns `Ok(RunId::new(12345))`

### Behavior: `parse_run_id returns ValidationFailed when input is non-numeric`
Given: no preconditions
When: `parse_run_id("not-a-number")`
Then: returns `Err(CliExitCode::ValidationFailed)`

### Behavior: `parse_run_id returns ValidationFailed when input is out-of-range decimal`
Given: no preconditions
When: `parse_run_id("99999999999999999999")` (exceeds u64::MAX)
Then: returns `Err(CliExitCode::ValidationFailed)`

### Behavior: `handle emits AiContextPacket JSON when run exists with events`
Given: a Fjall journal at `/tmp/test.db` containing a run with `RunAccepted` and `RunFinished` events
When: `handle("1", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: stdout receives a JSON object with `schema_version == "1"`, `kind == "AiContextPacket"`, `run_id == 1`, `journal_event_trail` is an array, `action_contracts` is an array, `suggested_next_cli_commands` is an array, `trace_ring_snapshot` is an object

### Behavior: `handle returns ValidationFailed with RUN_NOT_FOUND when run has zero events`
Given: a Fjall journal at `/tmp/test.db` with a run header but no journal events
When: `handle("999", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: exit code is `CliExitCode::ValidationFailed`, stderr receives JSON with `"code": "RUN_NOT_FOUND"`, `"success": false`

### Behavior: `handle returns StorageError when journal cannot be opened`
Given: path `/nonexistent/path`
When: `handle("1", Path::new("/nonexistent/path"), OutputFormat::Json)`
Then: exit code is `CliExitCode::StorageError`, stderr receives JSON with `"success": false` and `"error"` containing "opening journal"

### Behavior: `handle returns StorageError when events_for_run fails`
Given: a journal with a run whose event lookup returns an error
When: `handle("1", Path::new("/tmp/corrupt.db"), OutputFormat::Json)`
Then: exit code is `CliExitCode::StorageError`, stderr receives JSON with `"success": false` and `"error"` containing "reading events"

### Behavior: `redacted_slot_value returns [REDACTED] when slot is Secret (taint=1)`
Given: `SlotIdx(0)`, `Some(vec![1,2,3])`, `Some(snapshot with taint[0] = 1)`
When: `redacted_slot_value(slot, value, snapshot)`
Then: returns `Value::String("[REDACTED]")`

### Behavior: `redacted_slot_value returns [REDACTED] when slot is DerivedFromSecret (taint=2)`
Given: `SlotIdx(5)`, `Some(vec![9,8,7])`, `Some(snapshot with taint[5] = 2)`
When: `redacted_slot_value(slot, value, snapshot)`
Then: returns `Value::String("[REDACTED]")`

### Behavior: `redacted_slot_value returns decoded SlotValue string when slot is Clean`
Given: a valid `SlotValue` encoded bytes and `snapshot` with taint[slot] = 0
When: `redacted_slot_value(slot, Some(valid_bytes), snapshot)`
Then: returns `Value::String(slot_value.to_string())` — raw bytes never appear in output

### Behavior: `redacted_slot_value returns [UNDECODED] when bytes fail postcard decode`
Given: `SlotIdx(0)`, `Some(invalid_utf8_bytes)`, `snapshot with taint[0] = 0`
When: `redacted_slot_value(slot, value, snapshot)`
Then: returns `Value::String("[UNDECODED]")` — raw bytes never appear in output

### Behavior: `redacted_slot_value returns Null when value is None and slot is Clean`
Given: `SlotIdx(0)`, `None`, `snapshot with taint[0] = 0`
When: `redacted_slot_value(slot, None, snapshot)`
Then: returns `Value::Null`

### Behavior: `slot_is_secret_or_derived returns true when taint entry is 1`
Given: `SlotIdx(0)`, `snapshot with taint[0] = 1`
When: `slot_is_secret_or_derived(slot, snapshot)`
Then: returns `true`

### Behavior: `slot_is_secret_or_derived returns false when taint entry is 0`
Given: `SlotIdx(0)`, `snapshot with taint[0] = 0`
When: `slot_is_secret_or_derived(slot, snapshot)`
Then: returns `false`

### Behavior: `slot_is_secret_or_derived returns false when snapshot is None`
Given: `SlotIdx(0)`, `None`
When: `slot_is_secret_or_derived(slot, None)`
Then: returns `false`

### Behavior: `suggested_ai_commands returns inspect and events for all statuses`
Given: `RunStatus::Running`, `RunStatus::Finished`, `RunStatus::Failed`, `RunStatus::Cancelled`
When: `suggested_ai_commands("1", Path::new("/tmp/db"), status)`
Then: first two entries are exactly `"velvet-ballastics inspect 1 --db /tmp/db --json"` and `"velvet-ballastics events 1 --db /tmp/db --json"`

### Behavior: `suggested_ai_commands adds incident and retry when status is Failed`
Given: `RunStatus::Failed`
When: `suggested_ai_commands("1", Path::new("/tmp/db"), status)`
Then: returns 4 entries; entries 3 and 4 are `"velvet-ballastics incident 1 --db /tmp/db --json"` and `"velvet-ballastics retry 1 --db /tmp/db --json"`

### Behavior: `suggested_ai_commands adds trace and resume when status is Running`
Given: `RunStatus::Running`
When: `suggested_ai_commands("1", Path::new("/tmp/db"), status)`
Then: returns 4 entries; entries 3 and 4 are `"velvet-ballastics trace 1 --db /tmp/db --json"` and `"velvet-ballastics resume 1 --db /tmp/db --json"`

### Behavior: `suggested_ai_commands adds replay when status is Finished`
Given: `RunStatus::Finished`
When: `suggested_ai_commands("1", Path::new("/tmp/db"), status)`
Then: returns 3 entries; entry 3 is `"velvet-ballastics replay 1 --db /tmp/db --json"`

### Behavior: `suggested_ai_commands returns max 4 commands`
Given: any `RunStatus`
When: `suggested_ai_commands("1", Path::new("/tmp/db"), status)`
Then: `result.len() <= 4`

### Behavior: `suggested_ai_commands all commands start with velvet-ballastics`
Given: any `RunStatus`
When: `suggested_ai_commands("1", Path::new("/tmp/db"), status)`
Then: every string in the result starts with `"velvet-ballastics "`

### Behavior: `ai_workflow_summary returns null digest when no digest available`
Given: `journal`, `None` digest
When: `ai_workflow_summary(journal, None)`
Then: result has `"digest": null`, `"compiled_ir": {"available": false, ...}`, `"source_included": false`

### Behavior: `ai_workflow_summary returns compiled_ir available true with full node data`
Given: a journal with a compiled workflow record for the digest
When: `ai_workflow_summary(journal, Some(digest))`
Then: result has `"digest": "<hex>"`, `"compiled_ir": {"available": true, "name": ..., "node_count": ..., "slot_count": ..., "nodes": [...]}`, `"referenced_actions": [...]`

### Behavior: `ai_action_contracts returns unique action IDs from events`
Given: events containing `ActionScheduled { action: 5 }` and `ActionScheduled { action: 5 }` (duplicate) and `ActionCompletedEvent { action: 7 }`
When: `ai_action_contracts(events, None)`
Then: returns array with two entries: action 5 and action 7, each having `"contract_status": "inferred_from_compiled_ir_and_journal"`

### Behavior: `run_status_from_events returns Finished when last event is RunFinished`
Given: events ending with `JournalEvent::RunFinished { .. }`
When: `run_status_from_events(events)`
Then: returns `RunStatus::Finished`

### Behavior: `run_status_from_events returns Failed when last event is RunFailedEvent`
Given: events ending with `JournalEvent::RunFailedEvent { .. }`
When: `run_status_from_events(events)`
Then: returns `RunStatus::Failed`

### Behavior: `run_status_from_events returns Cancelled when last event is RunCancelled`
Given: events ending with `JournalEvent::RunCancelled { .. }`
When: `run_status_from_events(events)`
Then: returns `RunStatus::Cancelled`

### Behavior: `run_status_from_events returns Running otherwise`
Given: events ending with `JournalEvent::StepSucceeded { .. }`
When: `run_status_from_events(events)`
Then: returns `RunStatus::Running`

### Behavior: `run_status_from_events returns Running when events list is empty`
Given: an empty events list `[]`
When: `run_status_from_events(events)`
Then: returns `RunStatus::Running`

### Behavior: `parse_run_id returns ValidationFailed when input is empty string`
Given: no preconditions
When: `parse_run_id("")`
Then: returns `Err(CliExitCode::ValidationFailed)`

### Behavior: `parse_run_id returns ValidationFailed when input has whitespace prefix`
Given: no preconditions
When: `parse_run_id(" 12345")`
Then: returns `Err(CliExitCode::ValidationFailed)`

### Behavior: `redacted_slot_value returns Null when taint=0 and value is empty Vec<u8>`
Given: `SlotIdx(0)`, `Some(vec![])`, `snapshot with taint[0] = 0`
When: `redacted_slot_value(slot, value, snapshot)`
Then: returns `Value::Null`

### Behavior: `ai_workflow_summary returns compiled_ir available false when IR decode fails`
Given: a journal with a compiled workflow record for the digest but the IR bytes fail to decode
When: `ai_workflow_summary(journal, Some(digest))`
Then: result has `"digest": "<hex>"`, `"compiled_ir": {"available": false, "reason": "decode failed"}`, `"source_included": true`

### Behavior: `handle emits [REDACTED] for secret-tainted slot values in output`
Given: a Fjall journal at `/tmp/test.db` containing a run with events whose slots are marked Secret (taint=1) or DerivedFromSecret (taint=2) in the snapshot
When: `handle("1", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: the JSON output contains `[REDACTED]` strings in `journal_event_trail` entries for those slots; no raw bytes appear in output

### Behavior: `handle resolves workflow digest to available compiled IR`
Given: a Fjall journal at `/tmp/test.db` containing a run with a workflow digest that resolves to available compiled IR
When: `handle("1", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: `workflow.digest` is populated, `workflow.compiled_ir.available` is `true`, `workflow.compiled_ir` contains `name`, `node_count`, `slot_count`, and `nodes`

### Behavior: `handle infers action IDs from both journal events and compiled IR`
Given: a Fjall journal at `/tmp/test.db` containing a run with both `ActionScheduled` events and a compiled workflow with `Do` nodes
When: `handle("1", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: `action_contracts` contains action IDs from both sources, each with `contract_status: "inferred_from_compiled_ir_and_journal"`, and no duplicate action IDs

### Behavior: `handle returns StorageError when journal file is corrupt`
Given: a corrupt Fjall journal file at `/tmp/corrupt.db` (invalid binary data)
When: `handle("1", Path::new("/tmp/corrupt.db"), OutputFormat::Json)`
Then: exit code is `CliExitCode::StorageError`, stderr receives JSON with `"success": false` and `"error"` containing "reading events"

### Behavior: `handle degrades gracefully when latest_snapshot returns None`
Given: a Fjall journal at `/tmp/test.db` containing a run where `latest_snapshot` returns `None`
When: `handle("1", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: `trace_ring_snapshot` is `null` in output; slot values in `journal_event_trail` render as decoded strings or `[UNDECODED]` (never `[REDACTED]` since taint is unavailable); exit code is `0`

### Behavior: `handle returns StorageError when run_header lookup fails`
Given: a Fjall journal at `/tmp/test.db` where `run_header` lookup for the run ID returns an error
When: `handle("1", Path::new("/tmp/test.db"), OutputFormat::Json)`
Then: exit code is `CliExitCode::StorageError`, stderr receives JSON with `"success": false` and `"error"` containing "run header"

---

## Section 4 — Proptest Invariants

### `redacted_slot_value` (POST-003, POST-003-KANI, INV-004)
**Property 1**: Secret/derived slots always return `[REDACTED]`
- Strategy: `(slot: SlotIdx, bytes: Option<Vec<u8>>, taint_value: u8)` where `taint_value in {0, 1, 2}` and when `taint_value in {1, 2}` the snapshot is `Some(snapshot_with_taint_at(slot, taint_value))`
- Assert: `redacted_slot_value(slot, bytes, snapshot) == Value::String("[REDACTED]".to_string())`

**Property 2**: Clean slots never return `[REDACTED]`
- Strategy: `(slot: SlotIdx, bytes: Option<Vec<u8>>)` with `snapshot` having `taint[slot] = 0`
- Assert: `redacted_slot_value(slot, bytes, snapshot) != Value::String("[REDACTED]".to_string())`

**Property 3**: Decode failure emits `[UNDECODED]`, never raw bytes
- Strategy: `arbitrary_bytes()` that is NOT a valid `SlotValue` postcard encoding
- Assert: result is either `Value::String("[UNDECODED]")` or `Value::Null` or clean decoded value — never contains raw input bytes

### `parse_run_id` (PRE-001, ERR-INVALID-RUN-ID)
**Property**: Arbitrary string input either parses as valid `u64` or returns `ValidationFailed`
- Strategy: `any::<String>()`
- Assert: either `parse_run_id(s).is_ok()` (when `s` is valid decimal `u64`) or `parse_run_id(s).is_err()` (all other strings)

### `suggested_ai_commands` (INV-002, POST-004-LEN)
**Property 1**: Output length is bounded by run status (max 4)
- Strategy: `(run_id: String, db_path: String, status: RunStatus)` 
- Assert: `result.len() <= 4`

**Property 2**: All commands start with `velvet-ballastics`
- Strategy: same as above
- Assert: `result.iter().all(|cmd| cmd.starts_with("velvet-ballastics "))`

---

## Section 5 — Fuzz Targets

### `redacted_slot_value` fuzz target (INV-004, POST-003)
- **Target function**: `redacted_slot_value(slot: vb_core::SlotIdx, value: Option<Vec<u8>>, snapshot: Option<vb_storage::RunSnapshot>)`
- **Risk**: Malformed `Vec<u8>` payloads fed to `postcard::from_bytes::<vb_core::SlotValue>` could cause panics or leak raw secret content in error messages
- **Corpus seeds**: 
  - Valid `SlotValue` encoded bytes (collected from known-good journal event serializations)
  - Empty `vec![]`
  - Random 1-byte, 2-byte, 4-byte, 8-byte payloads
  - UTF-8 non-ASCII strings
  - Binary garbage (0x00–0xFF random)
- **Harness**: `cargo fuzz run -p velvet_ballastics redacted_slot_value_fuzz corpus/` — expect zero panics, zero raw byte leaks in output strings

### `ai_event_to_json` fuzz target
- **Target function**: `ai_event_to_json(event: &vb_storage::JournalEvent, snapshot: Option<&vb_storage::RunSnapshot>)`
- **Risk**: Binary journal records could deserialize into events that serialize incorrectly
- **Corpus seeds**: Valid `JournalEvent` binary encodings from journal compaction runs
- **Harness**: Verify output is valid `serde_json::Value` and all slot values are either `[REDACTED]`, `[UNDECODED]`, `null`, or a decoded string — never raw bytes

---

## Section 6 — Kani Harnesses

### `redacted_slot_value_kani` (POST-003-KANI)
**Property**: `redacted_slot_value` never panics for any `SlotIdx` (0..u16::MAX) and any `Option<Vec<u8>>`
**Bound**: `SlotIdx` range: 0..65535 (u16::MAX). `Vec<u8>` length bounded by test corpus max (256 bytes) to keep proof tractable.
**Rationale**: `slot_is_secret_or_derived` indexes into `snapshot.taint` via `slot.as_usize()`. Without bounds checking, an out-of-bounds slot index could panic. Kani proves this cannot happen.

### `slot_is_secret_or_derived_kani`
**Property**: `slot_is_secret_or_derived` returns deterministic bool for any `SlotIdx` and any `Option<RunSnapshot>` — never panics
**Bound**: Same as above
**Rationale**: Partial application of taint lookup; must not index out of bounds

---

## Section 7 — Mutation Testing Checkpoints

| Mutant | Location | Kill Strategy |
|--------|----------|---------------|
| Remove `[REDACTED]` guard in `redacted_slot_value` | line 337-338 | `redacted_slot_value_property` proptest — secret/derived inputs return non-redacted string; assertion fails |
| Change `matches!(*raw, 1 \| 2)` to `matches!(*raw, 1)` | line 354 | `slot_is_secret_or_derived_property` — derived slots (taint=2) return false; assertion fails |
| Remove `slot_is_secret_or_derived` call entirely | line 337 | `redacted_slot_value_property` — clean slots also redact; assertion fails |
| Change `base.into_iter()` to `base.iter().cloned()` in `suggested_ai_commands` | lines 428-429 | `suggested_ai_commands_length_bounded` — compile error or runtime panic on owned `String` |
| Change `RunStatus::Failed \| RunStatus::Cancelled` arm to only Failed | line 433 | `suggested_ai_commands_status_cancelled` unit test — cancelled run gets wrong commands |
| Remove `.or_else(|| workflow_digest_from_events(&events))` | line 50 | `ai_workflow_summary_with_digest_from_event` integration test — missing digest in packet |
| Remove `push_unique_u32` deduplication in `ai_action_contracts` | line 377 | `ai_action_contracts_duplicates` — duplicate action IDs appear in output |
| MUTATIONS-002: Remove `report_run_not_found` call in `handle` line 34 | line 34 | `run_not_found_on_empty_events` integration test — exit code is SUCCESS instead of ValidationFailed, JSON lacks `RUN_NOT_FOUND` code |
| Change `[UNDECODED]` string to raw bytes in `redacted_slot_value` | line 342 | `redacted_slot_value_UNDECODED_no_leak` — raw bytes appear in JSON output |
| Skip `journal_event_trail` population loop silently | lines 52-68 | `handle emits [REDACTED] for secret-tainted slot values` integration test — asserts `journal_event_trail.len() > 0` and entries have non-null slot_data fields |

**Target kill rate**: ≥90%

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| parse_run_id: valid | `"12345"` | `Ok(RunId::new(12345))` | unit |
| parse_run_id: invalid string | `"abc"` | `Err(ValidationFailed)` | unit |
| parse_run_id: overflow | `"18446744073709551616"` | `Err(ValidationFailed)` | unit |
| parse_run_id: empty | `""` | `Err(ValidationFailed)` | unit |
| parse_run_id: whitespace prefix | `" 12345"` | `Err(ValidationFailed)` | unit |
| handle: valid run | real journal with events | JSON packet with all fields populated | integration |
| handle: secret redaction | run with taint=1/2 slots | `[REDACTED]` appears in output, no raw bytes | integration |
| handle: workflow digest IR | run with available compiled IR | `compiled_ir.available: true` with node data | integration |
| handle: action inference both sources | run with events AND compiled IR | unique action IDs from both sources | integration |
| handle: run not found | run ID with no events | `RUN_NOT_FOUND` JSON + ValidationFailed | integration |
| handle: journal open fail | `/nonexistent` | `StorageError` JSON + StorageError | manual-qa |
| handle: journal read fail | corrupted journal | `StorageError` JSON + StorageError | integration |
| handle: run_header fail | run header lookup error | `StorageError` JSON + StorageError | integration |
| handle: snapshot None | latest_snapshot returns None | `trace_ring_snapshot: null`, exit 0 | integration |
| redacted: secret slot | taint=1 | `[REDACTED]` | unit |
| redacted: derived slot | taint=2 | `[REDACTED]` | unit |
| redacted: clean slot | taint=0 + valid bytes | decoded string | unit |
| redacted: undecodable | taint=0 + invalid bytes | `[UNDECODED]` | unit + fuzz |
| redacted: None value | taint=0 + None | `Null` | unit |
| redacted: empty vec | taint=0 + empty vec | `Null` | unit |
| slot_is_secret_or_derived: taint=0 | snapshot with taint[0]=0 | `false` | unit |
| slot_is_secret_or_derived: taint=1 | snapshot with taint[0]=1 | `true` | unit |
| slot_is_secret_or_derived: taint=2 | snapshot with taint[0]=2 | `true` | unit |
| slot_is_secret_or_derived: None snapshot | `None` | `false` | unit |
| suggested: Running | RunStatus::Running | 4 commands (inspect, events, trace, resume) | unit |
| suggested: Finished | RunStatus::Finished | 3 commands (inspect, events, replay) | unit |
| suggested: Failed | RunStatus::Failed | 4 commands (inspect, events, incident, retry) | unit |
| suggested: Cancelled | RunStatus::Cancelled | 4 commands (inspect, events, incident, retry) | unit |
| suggested: length bound | any status | `len() <= 4` | proptest |
| suggested: command prefix | any status | all start with `velvet-ballastics` | proptest |
| workflow: no digest | digest=None | `digest: null, compiled_ir.available: false` | unit |
| workflow: with digest, IR available | real journal | `digest: <hex>, compiled_ir.available: true, nodes: [...]` | integration |
| workflow: IR decode fail | corrupted IR bytes | `compiled_ir.available: false, reason: "decode failed"` | unit |
| action_contracts: no events | empty events | `[]` | unit |
| action_contracts: with events | events with actions | unique action IDs with inferred status | unit |
| action_contracts: both sources | events + compiled IR | union of unique IDs with inferred status | integration |
| run_status: RunFinished | last=RunFinished | `Finished` | unit |
| run_status: RunFailedEvent | last=RunFailedEvent | `Failed` | unit |
| run_status: RunCancelled | last=RunCancelled | `Cancelled` | unit |
| run_status: other | last=StepSucceeded | `Running` | unit |
| run_status: empty events | `[]` | `Running` | unit |
| invariant: read-only | handle execution | no mutable journal writes | static-scan |
| invariant: bounded packet | suggested commands | max 4 entries | proptest |
| invariant: all real commands | suggested commands | all map to real subcommands | static-scan + manual-qa |
| invariant: trail cardinality | handle with events | `journal_event_trail.len() > 0` | integration |

---

## Section 9 — Proof Obligation Coverage

| Obligation ID | Test Function | Layer | Tool |
|---------------|---------------|-------|------|
| PRE-001 | `parse_run_id_returns_run_id_when_valid_decimal_u64` + `parse_run_id_generated` (proptest) | unit + proptest | cargo nextest + proptest |
| PRE-002 | `velvet-ballastics ai-context 1 --db /nonexistent/path --json` | manual-qa | manual invocation |
| PRE-003 | `run_not_found_on_empty_events` | integration | cargo nextest |
| POST-001 | `ai_context_packet_schema_fields` + `secret_redaction_in_output` + `action_ids_from_both_sources` | integration | cargo nextest |
| POST-002 | `workflow_field_populated` + `workflow_digest_resolves_to_ir` | integration | cargo nextest |
| POST-003 | `redacted_slot_value_property` (secret/derived always redact) | proptest | cargo nextest |
| POST-003-KANI | `redacted_slot_value_kani` | kani | cargo kani |
| POST-004 | `slot_is_secret_or_derived_property` + `suggested_commands_all_real` | proptest + static-scan | cargo nextest + clippy |
| POST-004-LEN | `suggested_commands_length_bounded` | proptest | cargo nextest |
| POST-005 | `action_contracts_inferred` + `action_ids_from_both_sources` | integration | cargo nextest |
| POST-006 | `run_not_found_structured_error` | integration | cargo nextest |
| INV-001 | static-scan: no mutable journal writes | static-scan | cargo machete + code review |
| INV-002 | `suggested_commands_length_bounded` | proptest | cargo nextest |
| INV-003 | `suggested_commands_all_real` + `velvet-ballastics help` diff | static-scan + manual-qa | clippy + manual |
| INV-004 | `redacted_slot_value_UNDECODED_no_leak` + `cargo fuzz` | fuzz | cargo-fuzz |
| ERR-INVALID-RUN-ID | `invalid_run_id_rejected` | unit | cargo nextest |
| ERR-JOURNAL-OPEN | `velvet-ballastics ai-context 1 --db /invalid/db/path --json` | manual-qa | manual invocation |
| ERR-RUN-NOT-FOUND | `run_not_found_on_empty_events` | integration | cargo nextest |
| ERR-JOURNAL-READ | `journal_read_error_propagates` + `corrupt_journal_error` | integration | cargo nextest |
| MUTATIONS-001 | `redacted_slot_value_property` kills redaction removal mutants | mutation | cargo mutants |
| MUTATIONS-002 | `run_not_found_on_empty_events` kills `report_run_not_found` call removal | mutation | cargo mutants |
| COVERAGE-001 | `commands_ai_context.rs > 90%` line coverage | coverage | cargo llvm-cov |
| CLIPPY-001 | clippy clean on `commands_ai_context` | static | cargo clippy |
| GATE-CI | `moon run :ci` | gauntlet | moon |

All 26 proof obligations are addressed. Zero obligations are waived except as documented in verification-layers.md (Miri, Lean, Loom).

---

## Section 10 — Test Execution Order

```
# 1. Static analysis (fastest, catches obvious issues first)
cargo clippy -p velvet_ballastics --
cargo machete -p velvet_ballastics

# 2. Unit tests (pure functions, no I/O)
cargo nextest -p velvet_ballastics --test-threads=1 commands_ai_context::

# 3. Proptest (property-based, bounded exhaustive on small domains)
cargo nextest -p velvet_ballastics --test-threads=1 commands_ai_context::proptest

# 4. Integration tests (real Fjall journal)
cargo nextest -p velvet_ballastics --test-threads=1 ai_context_integration

# 5. Kani (formal verification, slow)
cargo kani --tests -p velvet_ballastics --harness redacted_slot_value_kani

# 6. Fuzz (mutation-based, runs indefinitely)
cargo fuzz run -p velvet_ballastics redacted_slot_value_fuzz corpus/

# 7. Mutation testing (post-pass, after all tests green)
cargo mutants -p velvet_ballastics -- --test-threads=1

# 8. Coverage gate
cargo llvm-cov -p velvet_ballastics --html 2>&1 | grep commands_ai_context

# 9. Manual QA (human-executed against live binary)
# (documented in manual-qa-notes.md)

# 10. Full CI gauntlet
moon run :ci
```
