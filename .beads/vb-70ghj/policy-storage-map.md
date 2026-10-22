# Compiled Policy Decision Storage/Recovery Consumer Map

Bead: vb-70ghj (child of vb-ny068)
Baseline: 5fff342a (unaccepted prototype)
Scope: Exact compiled callgraph for RuntimeJournalEvent::PolicyDecision through Fjall storage and recovery consumers.

## Compiled Callgraph

### 1. Runtime Event Creation
- `crates/vb_runtime/src/policy.rs:17` — `PolicyDecision` struct (runtime policy evaluation result)
- `crates/vb_runtime/src/policy.rs:113-139` — `PolicyDecisionRecorder::evaluate_action()` constructs `PolicyDecision` at outbox boundary
- `crates/vb_runtime/src/journal/chunk_001.rs:173-187` — `RuntimeJournalEvent::PolicyDecision` variant (runtime journal event wrapper)

### 2. Runtime → Storage Conversion
- `crates/vb_runtime/src/journal/chunk_002.rs:143-155` — `StorageRuntimeJournal::run_storage_event()` arm converts `RuntimeJournalEvent::PolicyDecision` → `JournalEvent::PolicyDecisionEvent`
- `crates/vb_runtime/src/journal/chunk_002.rs:362-408` — `StorageRuntimeJournal::storage_event()` dispatches to `run_storage_event()` for this variant
- Conversion passes all 8 fields: run, layer, code, policy_digest, input_digest, emergency_deny_active, compatibility_influence
- **seq field injected** at conversion time (not carried by runtime event)

### 3. Storage Event Definition
- `crates/vb_storage/src/events/event.rs:363-377` — `JournalEvent::PolicyDecisionEvent` (storage event variant)
- Fields: run, seq, layer, code, policy_digest, input_digest, emergency_deny_active, compatibility_influence
- `crates/vb_storage/src/events.rs:128` — `record_kind()` maps PolicyDecisionEvent → `RecordKind::PolicyDecision`

### 4. RecordKind Tag
- `crates/vb_storage/src/records.rs:239` — `RecordKind::PolicyDecision = 38`
- `crates/vb_storage/src/records.rs:286` — `RecordKind::PolicyDecision.id()` returns 38
- `crates/vb_storage/src/records.rs:338` — `RecordKind::from_id(38)` returns `Some(PolicyDecision)`
- `crates/vb_storage/src/records.rs:390` — `is_journal_event()` includes PolicyDecision (true)

### 5. Wire Serialization (CODEC) — **CRITICAL GAPS**

**Serialization routing:**
- `crates/vb_storage/src/events/wire_serialize.rs` — `serialize_by_category()` dispatches by `journal_record_category(kind)`
- `crates/vb_storage/src/events/wire_category.rs` — `journal_record_category(RecordKind::PolicyDecision)` returns **`None`**
  - `is_run_record()` (line 26-38) — does NOT include PolicyDecision
  - `is_step_record()` — does NOT include PolicyDecision
  - `is_action_record()` — does NOT include PolicyDecision
  - `is_wait_or_ask_record()` — does NOT include PolicyDecision
- **Result:** `serialize_by_category()` hits `serialize_routing_error()` at runtime for PolicyDecisionEvent

**Missing serialization components:**
- `crates/vb_storage/src/events/wire_tags.rs` — No `TAG_POLICY_DECISION` constant defined
- `crates/vb_storage/src/events/wire_serialize_run.rs` — No `serialize_policy_decision()` function
- `crates/vb_storage/src/events/wire_deserialize_run.rs` — No `deserialize_policy_decision()` function
- `crates/vb_storage/src/events/wire_tags.rs` — `JOURNAL_EVENT_VARIANTS` array does NOT include "PolicyDecisionEvent"
- `crates/vb_storage/src/events/wire_legacy*.rs` — No legacy codec path for PolicyDecisionEvent

### 6. Recovery Consumers (handle PolicyDecision correctly)

| Consumer | File:Line | Behavior |
|----------|-----------|----------|
| Hydration | `recovery/hydrate.rs:815` | Returns `Ok(false)` (no state change) |
| Incident | `journal/incident.rs:211` | Maps to `LifecycleState::Active` |
| Replay Summary | `recovery/replay/summary/apply.rs:85` | No-op (excluded from event log ordering) |
| events.rs run_id() | `events.rs:51` | Extracts run from PolicyDecisionEvent |
| events.rs seq() | `events.rs:87` | Extracts seq from PolicyDecisionEvent |
| events.rs record_kind() | `events.rs:128` | Maps to RecordKind::PolicyDecision |
| events.rs is_terminal() | `events.rs:205` | Not terminal (false) |
| events.rs has_seq() | `events.rs:256` | Has seq (true) |

### 7. Missing Identity Fields vs Required

**Current PolicyDecision struct fields (policy.rs:17-28):**
- run: RunId ✓
- layer: PolicyLayer ✓
- code: DecisionCode ✓
- policy_digest: PolicyDigest ✓
- input_digest: PolicyDigest (computed from ticket, not stored)
- emergency_deny_active: bool ✓
- compatibility_influence: bool ✓

**Missing from PolicyDecision (required by spec):**
- step: StepIdx — NOT carried (only in input_digest)
- action: ActionId — NOT carried (only in input_digest)
- attempt: u16 — NOT carried (only in input_digest)
- ticket identity fields are hashed into input_digest but not persisted individually
- **admission_policy_digest** — not stored in PolicyDecision (only live_effect_policy_digest via policy_digest field)
- The `PolicyDecisionRecorder` maintains `admission_policy_digest` but `evaluate_action()` only stores `live_effect_policy_digest` in the decision record

**Impact:** Crash/reopen can reconstruct that a policy decision was made for a run, but cannot identify which specific action ticket triggered it without the full ticket context. The input_digest is a blake3 hash of the ticket fields (policy.rs:106-116), so it's deterministic but not invertible.

### 8. Existing Durable Fjall Tests/Public API for Adaptation

**Best candidates for policy decision roundtrip tests:**

1. `crates/workspace_tests/benches/vb_h6ix_replay.rs` — Creates `JournalEvent` variants directly, appends to `FjallJournal`, reads back. Demonstrates the exact pattern:
   - Create events: `JournalEvent::ActionScheduled { run, seq, step, action, attempt }`
   - Append: `journal.append_journaled(&event)`
   - Read back: `journal.read_events(run, offset, count)`
   - Verify decoded events match

2. `crates/workspace_tests/benches/velvet_ballistics.rs` (line 557) — Tests postcard encode/decode roundtrip for `JournalEvent`:
   - `postcard::to_alloc_vec(&event)` then `postcard::from_bytes::<JournalEvent>(bytes)`
   - Already tests `RecordKind` discriminant preservation

3. `crates/workspace_tests/benches/vb_qi37_1_1_recovery.rs` — Tests recovery hydration with mixed event sequences

4. `crates/vb_storage/tests/recovery_property_tests.rs` — Uses `JournalEvent::RunFinished` and `RunFailedEvent` directly with hydration

**Public API used by all:**
- `vb_storage::FjallJournal::open(path)`
- `journal.append_journaled(&JournalEvent)` / `append_strict(&JournalEvent)`
- `journal.read_events(run_id, offset, count)`
- `vb_storage::summarize_recovery_events(&events)` for replay
- `postcard::to_alloc_vec()` / `postcard::from_bytes()` for codec testing

### Summary of Findings

| Area | Status | Evidence |
|------|--------|----------|
| Runtime event variant | ✓ Defined | chunk_001.rs:173 |
| Runtime→Storage conversion | ✓ Defined | chunk_002.rs:143 |
| Storage event variant | ✓ Defined | events/event.rs:363 |
| RecordKind tag (38) | ✓ Defined | records.rs:239 |
| Serialization routing | ✗ **BREAKS** | wire_category.rs returns None |
| Serialization function | ✗ Missing | No serialize_policy_decision() |
| Deserialization function | ✗ Missing | No deserialize_policy_decision() |
| Tag constant | ✗ Missing | No TAG_POLICY_DECISION |
| Variant name in array | ✗ Missing | Not in JOURNAL_EVENT_VARIANTS |
| Recovery hydration | ✓ Handles | hydrate.rs:815 |
| Incident mapping | ✓ Handles | incident.rs:211 |
| Replay summary | ✓ Handles | apply.rs:85 |
| Ticket identity fields | ✗ Missing | step/action/attempt not persisted |
| Admission policy digest | ✗ Missing | Only live_effect stored |

The new variant **does not** preserve old postcard enum numbering because it uses the new RecordKind tag-based serialization (tag=38). However, the RecordKind-based serialization path is broken for this variant because it's not registered in `journal_record_category()`.
