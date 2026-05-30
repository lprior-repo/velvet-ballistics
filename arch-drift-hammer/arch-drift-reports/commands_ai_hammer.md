# Architectural Drift Report: `commands_ai_context.rs`

**File:** `crates/vb_cli/src/commands_ai_context.rs`  
**Line count:** 699 (VIOLATION: >300 lines)  
**Status:** `REFACTOR REQUIRED`

---

## 1. LINE COUNT VIOLATION

**Rule:** No file may exceed 300 lines.  
**Actual:** 699 lines  
**Violation:** 133% over the limit

---

## 2. RESPONSIBILITY MAP

The file conflates **6 distinct command concerns** that must be separated:

| # | Responsibility | Lines | New File |
|---|---------------|-------|----------|
| 1 | **Entry point + orchestration** (`handle`) | 21–78 | `ai_context_cmd.rs` |
| 2 | **Run ID parsing + validation** | 80–107 | `run_id.rs` (NewType) |
| 3 | **Error reporting** (4 functions) | 109–163 | `ai_context_errors.rs` |
| 4 | **Workflow summary + decode** (11 functions) | 165–360 | `ai_workflow.rs` |
| 5 | **Journal event → JSON serialization** | 362–510, 553–643 | `ai_journal_serde.rs` |
| 6 | **Node kind naming** | 513–551 | (method on `CompiledNodeKind`) |
| 7 | **Output utilities** (`write_stderr_*`, `json_error`) | 645–674 | `cli_output.rs` |
| 8 | **Unit test** | 676–699 | `ai_context_test.rs` (inline `mod tests`) |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### VIOLATION A: `action: u32` (line 331–338, 414–438, 441–458)

**Problem:** `u32` is used for `ActionId` throughout. No NewType wrapper.

```rust
// Line 331-338 — raw u32
fn referenced_actions(compiled: &vb_core::CompiledWorkflow) -> Vec<u32> {
    (0..compiled.node_count())
        .filter_map(|raw| compiled.node(vb_core::StepIdx::new(raw)))
        .filter_map(|node| match &node.kind {
            vb_core::workflow::CompiledNodeKind::Do { action, .. } => Some(u32::from(action.get())),
            _ => None,
        })
        .fold(Vec::<u32>::new(), push_unique_u32)
}

// Line 414-438 — raw u32 with parse-don't-validate anti-pattern
fn ai_action_contracts(
    events: &[vb_storage::JournalEvent],
    workflow_actions: Option<&Value>,
) -> Value {
    let workflow_ids = workflow_actions
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_u64().and_then(|raw| u32::try_from(raw).ok()));  // ← parse, don't validate

    let event_ids = events.iter().filter_map(|event| match event {
        vb_storage::JournalEvent::ActionScheduled { action, .. }
        | vb_storage::JournalEvent::ActionCompletedEvent { action, .. }
        | vb_storage::JournalEvent::ActionFailedEvent { action, .. } => {
            Some(u32::from(action.get()))  // ← unwrap without bounds check
        }
        _ => None,
    });
    Value::Array(
        workflow_ids
            .chain(event_ids)
            .fold(Vec::<u32>::new(), push_unique_u32)
            .into_iter()
            .map(inferred_action_contract_json)  // ← u32 → JSON with no validation
            .collect(),
    )
}

// Line 441-458 — u32 baked into JSON structure
fn inferred_action_contract_json(action: u32) -> Value {
    serde_json::json!({
        "action": action,  // ← raw integer, no type abstraction
        "contract_status": "inferred_from_compiled_ir_and_journal",
        "contract": {
            "id": action,  // ← duplicative field
            "source": "compiled_ir_do_node_or_action_event",
            "input_slot_count": null,
            "output_slot_count": null,
            ...
        }
    })
}
```

**Scott Wlaschin fix:** Introduce `ActionId(u32)` NewType. `inferred_action_contract_json` becomes `impl From<ActionId> for Value`.

---

### VIOLATION B: `run_id: &str` (lines 479–511)

**Problem:** `suggested_ai_commands` takes `run_id: &str` and formats it into CLI strings via raw `format!`. This is primitive obsession — `RunId` should be a typed value that knows how to render itself into CLI args.

```rust
// Line 479-511
pub(crate) fn suggested_ai_commands(
    run_id: &str,       // ← primitive obsession
    db: &std::path::Path,
    status: RunStatus,
) -> Vec<String> {
    let db_arg = db.display();
    let base = vec![
        format!("velvet-ballistics inspect {run_id} --db {db_arg} --emit yaml"),  // ← stringly-typed CLI
        format!("velvet-ballistics events {run_id} --db {db_arg} --emit yaml"),
    ];
    ...
}
```

**Scott Wlaschin fix:** `RunId` should be a NewType with a method `to_cli_args()` returning an iterator of `&str`. Commands should be built via a `CliCommand` builder, not string interpolation.

---

### VIOLATION C: `step: u16` / `raw: u16` (lines 318–329)

```rust
fn compiled_node_json(compiled: &vb_core::CompiledWorkflow, raw: u16) -> Option<Value> {
    let step = vb_core::StepIdx::new(raw);  // ← conversion at boundary, not enforced
    compiled.node(step).map(|node| {
        serde_json::json!({
            "step": raw,   // ← raw u16 leaks to JSON
            "name": compiled.step_name(step),
            "kind": node_kind_name(&node.kind),
            "output": node.output.map(|slot| slot.get()),
            "next": node.next.map(|next| next.get()),
        })
    })
}
```

**Fix:** `StepIdx` should serialize as JSON via `Serialize` impl, not `.get()`.

---

## 4. DDD PRINCIPLE VIOLATIONS

### VIOLATION D: `node_kind_name` is a PROCEDURAL FUNCTION, not a method (lines 513–551)

39 lines of procedural match against an enum variant. This should be a method on `CompiledNodeKind`:

```rust
// CURRENT (procedural, violates DDD)
fn node_kind_name(kind: &vb_core::workflow::CompiledNodeKind) -> &'static str {
    match kind {
        vb_core::workflow::CompiledNodeKind::Nop => "Nop",
        vb_core::workflow::CompiledNodeKind::SetConst { .. } => "SetConst",
        // ... 25 more arms
    }
}
```

**Fix:** `impl CompiledNodeKind { fn kind_name(&self) -> &'static str }`

---

### VIOLATION E: `trace_ring_snapshot()` always returns fabricated unavailable (lines 461–468)

```rust
fn trace_ring_snapshot() -> Value {
    serde_json::json!({
        "available": false,
        "reason": "TraceRing is volatile in-memory runtime state; this packet does not fabricate a durable trace snapshot",
        "fabricated": false,
        "events": []
    })
}
```

This is dead code that always lies. Either implement it properly or remove it entirely.

---

### VIOLATION F: `push_unique_u32` is mutable accumulation, not functional (lines 350–355)

```rust
fn push_unique_u32(mut values: Vec<u32>, value: u32) -> Vec<u32> {
    if !values.contains(&value) {
        values.push(value);
    }
    values
}
```

**Violations:**
- Mutating `values` in place instead of returning a new collection
- `O(n)` lookup per insert instead of using `HashSet`

**Fix:** Return `Vec<u32>` from iterator without mutation, or use `HashSet`.

---

### VIOLATION G: `ai_event_to_json` has DROPPED FIELDS (lines 367–387)

```rust
fn ai_event_to_json(
    event: &vb_storage::JournalEvent,
    snapshot: Option<&vb_storage::RunSnapshot>,
) -> Value {
    let value = event_to_json(event);
    match (event, value) {
        (
            vb_storage::JournalEvent::SlotWrittenEvent {
                slot, value: bytes, ..   // ← .. drops all other fields!
            },
            Value::Object(object),
        ) => Value::Object(Map::from_iter(object.into_iter().chain([
            ("slot".to_string(), Value::from(slot.get())),
            (
                "value".to_string(),
                redacted_slot_value(*slot, bytes.as_ref(), snapshot),
            ),
        ]))),
        (_, value) => value,
    }
}
```

The `..` in `SlotWrittenEvent` pattern silently drops `seq` and other fields. Compare to `event_to_json` at line 594 which also uses `..` for `SlotWrittenEvent { seq, slot, .. }`. These drops are inconsistent and lossy.

---

## 5. REFACTOR PRESCRIPTION

### Split into these modules:

```
crates/vb_cli/src/commands/   ← new commands/ submodule
├── mod.rs                    ← re-exports
├── ai_context/
│   ├── mod.rs               ← handle() + RunStatus
│   ├── run_id.rs            ← RunId parsing + validation
│   ├── errors.rs            ← error reporting functions
│   ├── workflow.rs          ← workflow summary + decode pipeline
│   ├── journal_serde.rs     ← ai_event_to_json, event_to_json, node_kind_name
│   ├── snapshot.rs          ← latest_snapshot_* functions
│   └── action_contracts.rs  ← ai_action_contracts, inferred_action_contract_json
└── output.rs                 ← write_stderr_*, json_error
```

### Required NewType wrappers to introduce:

| NewType | Underlying | Where used |
|---------|-----------|-----------|
| `ActionId(u32)` | `u32` | `referenced_actions`, `ai_action_contracts`, `inferred_action_contract_json` |
| `CliRunId(&str)` | `&str` | `suggested_ai_commands` argument |
| `StepRaw(u16)` | `u16` | `compiled_node_json` argument |

---

## 6. SUMMARY

| Category | Count |
|----------|-------|
| Lines over limit | 399 |
| Primitive obsession violations | 3 (ActionId, RunId, StepIdx raw) |
| DDD method-vs-function violations | 1 (node_kind_name) |
| Dead/stub code | 1 (trace_ring_snapshot) |
| Mutable accumulator anti-patterns | 1 (push_unique_u32) |
| Silent field drops | 2 (SlotWrittenEvent patterns) |

**MANDATORY ACTIONS:**
1. Split file into ≤300 line chunks per responsibility
2. Introduce `ActionId(u32)` NewType with `From<ActionId> for Value`
3. Move `node_kind_name` to `impl CompiledNodeKind`
4. Replace `push_unique_u32` with functional iterator pattern
5. Remove or implement `trace_ring_snapshot`
