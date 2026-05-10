# vb_ipc Test Plan

## Crate: vb_ipc

**Current state**: 400 tests pass, 76 `assert!(false, ...)` tautologies, 152 `expect()` in test code, 69.37% coverage
**Target**: 0 tautologies, ≤20 `expect()` in integration tests, ≥5x density, coverage ≥70% handlers/dispatch/client

---

## 1. TAUTOLOGICAL TEST FIXES

### 1.1 `handlers.rs` — 48 tautologies (lines 1125, 1154, 1171, 1185, 1202, 1218, 1236, 1254, 1272, 1286, 1303, 1321, 1353, 1373, 1390, 1407, 1519, 1529, 1543, 1552, 1564, 1573, 1584, 1593, 1668, 1682, 1698, 1711, 1727, 1740, 1755, 1768, 1871, 1885, 1900, 1913, 1941, 1953)

**Pattern A — roundtrip tests that use `return` after assert!(false,)**:
These encode a valid payload, then decode it. The decode MUST succeed or postcard is broken.
```rust
// WRONG:
let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
let result = decode_payload::<crate::IpcPayload>(&encoded);
let Ok(decoded) = result else {
    assert!(false, "should decode CancelRun");  // ← unreachable
    return;
};
assert_eq!(decoded, payload);

// RIGHT:
let Ok(encoded) = postcard::to_allocvec(&payload) else { return };
let decoded = decode_payload::<crate::IpcPayload>(&encoded)
    .expect("postcard roundtrip must succeed for valid IpcPayload");
assert_eq!(decoded, payload);
```

**Files affected** (roundtrip tests):
- `decode_payload_roundtrips_cancel_run` (line 1154)
- `decode_payload_roundtrips_drain_trace` (line 1171)
- `decode_payload_roundtrips_shutdown` (line 1185)
- `decode_payload_roundtrips_list_events` (line 1202)
- `decode_payload_roundtrips_inspect_run` (line 1218)
- `decode_payload_roundtrips_answer_ask` (line 1236)
- `decode_payload_roundtrips_complete_action` (line 1254)
- `decode_payload_roundtrips_fail_action` (line 1272)
- `decode_payload_roundtrips_get_metrics` (line 1286)
- `decode_payload_roundtrips_list_runs` (line 1303)
- `decode_payload_roundtrips_submit_run` (line 1321)
- `submit_run_delegates_directly_to_runtime` (line 1519 → 1529)
- `get_workflow_graph_payload_roundtrips` (line 1543 → 1552)
- `verify_workflow_payload_roundtrips` (line 1564 → 1573)
- `get_taint_report_payload_roundtrips` (line 1584 → 1593)
- `submit_run_oversized_input_survives_decode_for_handler_check` (line 1668 → 1682)
- `submit_run_input_at_exact_cap_decodes` (line 1698 → 1711)
- `submit_run_inline_payload_roundtrips` (line 1727 → 1740)
- `complete_action_payload_roundtrips` (line 1755 → 1768)
- `list_runs_payload_roundtrips` (line 1871 → 1885)
- `drain_trace_payload_roundtrips` (line 1900 → 1913)
- `cancel_run_payload_roundtrips` (line 1941 → 1953)

**Pattern B — error-path assertions that use `assert!(false, ...)` in the unreachable branch**:
```rust
// WRONG:
match result {
    Err(IpcResponse::PayloadError { diagnostic, message }) => {
        assert!(!message.is_empty());
        assert_eq!(diagnostic, 0x300D);
    }
    other => {
        assert!(false, "expected PayloadError for garbage, got {other:?}"); // ← unreachable
    }
}

// RIGHT:
let result = decode_payload::<crate::IpcPayload>(garbage);
assert!(result.is_err(), "decode_payload should fail for garbage bytes");
let Err(IpcResponse::PayloadError { diagnostic, message }) = result else {
    unreachable!("expected PayloadError variant");
};
assert!(!message.is_empty());
assert_eq!(diagnostic, 0x300D);
```

**Files affected**:
- `decode_payload_returns_error_for_garbage_bytes` (line 1125)
- `decode_payload_returns_error_for_empty_bytes` (line 1136)

**Pattern C — error-path for payload variant mismatches** (lines 1353, 1373, 1390, 1407):
```rust
// WRONG:
match decoded {
    Ok(_) => unreachable!("garbage should not decode"),
    Err(IpcResponse::PayloadError { .. }) => { /* correct */ }
    other => { assert!(false, "expected PayloadError, got {other:?}"); }
}

// RIGHT:
let decoded = decode_payload::<crate::IpcPayload>(&garbage);
assert!(decoded.is_err());
let Err(IpcResponse::PayloadError { .. }) = decoded else {
    unreachable!("expected PayloadError variant");
};
```

---

### 1.2 `trace.rs` — 6 tautologies (lines 301, 320, 340, 357, 381, 409)

All are in `typed_events_response` and `count_response_trace` tests where the match arm is correct but the `other => assert!(false, ...)` is unreachable.

**Fix**: Replace all 6 with ` unreachable!("expected {Variant}, got {other:?}")`

**Functions**:
- `typed_events_response_returns_empty_for_no_events` (line 301)
- `typed_events_response_returns_all_events_from_sequence_zero` (line 320)
- `typed_events_response_filters_by_from_sequence` (line 340)
- `typed_events_response_filters_all_when_from_sequence_exceeds_count` (line 357)
- `typed_events_response_preserves_event_kind_mapping` (line 381)
- `count_response_trace_returns_count_out_of_range_for_exceeding_u32` (line 409)

---

### 1.3 `impl_tests.rs` — 10 tautologies (lines 252, 255, 354, 357, 430, 433, 672, 675, 915, 1304, 1319, 1322)

**Pattern** — integration tests where `assert!(false, ...)` is in the unreachable error branch:
```rust
// WRONG:
match decoded {
    Ok(IpcResponse::Healthy) => {}
    Ok(other) => { assert!(false, "expected Healthy, got {other:?}"); }
    Err(e) => { assert!(false, "decode failed: {e}"); }
}

// RIGHT:
assert!(decoded.is_ok(), "response should decode");
let Ok(IpcResponse::Healthy) = decoded else {
    unreachable!("expected Healthy variant");
};
```

**Functions** (lines 252, 255, 354, 357, 430, 433, 672, 675, 915, 1304, 1319, 1322):
- `health_response_frame_decodes`
- `bad_request_error_response_decodes`
- `runtime_error_response_decodes`
- `frame_error_response_decodes`
- `encoded_healthy_roundtrips_through_postcard`
- `verify_health_check_integration_returns_healthy`

---

### 1.4 `metrics.rs` — 9 tautologies (lines 118, 143, 168, 186, 203, 222, 253, 310, 388)

All are in roundtrip tests where postcard decode MUST succeed:
```rust
// WRONG:
let decoded: RuntimeMetrics = match postcard::from_bytes(&encoded) {
    Ok(d) => d,
    Err(_) => { assert!(false, "decoding should succeed"); return; }
};

// RIGHT:
let decoded: RuntimeMetrics = postcard::from_bytes(&encoded)
    .expect("RuntimeMetrics roundtrip must succeed");
```

**Functions**:
- `runtime_metrics_postcard_roundtrip` (line 118)
- `shard_metrics_postcard_roundtrip` (line 143)
- `shard_metrics_with_max_values_roundtrip` (line 168)
- `journal_metrics_postcard_roundtrip` (line 186)
- `ipc_metrics_postcard_roundtrip` (line 203)
- `aggregate_metrics_postcard_roundtrip` (line 222)
- `runtime_metrics_empty_shards_roundtrip` (line 253)
- `runtime_metrics_multiple_shards_roundtrip` (line 310)
- `runtime_metrics_all_max_values_roundtrip` (line 388)

---

### 1.5 `ids.rs` — 8 tautologies (lines 208, 219, 231, 244, 366, 377, 389, 402)

All are in roundtrip tests where postcard decode MUST succeed:
```rust
// WRONG:
let decoded: AskTicketId = match postcard::from_bytes(&encoded) {
    Ok(d) => d,
    Err(_) => { assert!(false, "decode should succeed"); return; }
};

// RIGHT:
let decoded: AskTicketId = postcard::from_bytes(&encoded)
    .expect("AskTicketId roundtrip must succeed");
```

**Functions**:
- `ask_ticket_id_serde_roundtrip` (line 208)
- `action_ticket_id_serde_roundtrip` (line 219)
- `ask_ticket_id_serde_roundtrip_boundary` (line 231)
- `action_ticket_id_serde_roundtrip_boundary` (line 244)
- `ask_ticket_id_ordering` (line 366)
- `action_ticket_id_ordering` (line 377)
- `ask_ticket_id_ord_for_special_values` (line 389)
- `action_ticket_id_ord_for_special_values` (line 402)

---

### 1.6 `action_output.rs` — 3 tautologies (lines 127, 146, 164)

```rust
// WRONG:
let decoded: IpcActionOutputPayload = match postcard::from_bytes(&encoded) {
    Ok(d) => d,
    Err(_) => { assert!(false, "decoding should succeed"); return; }
};

// RIGHT:
let decoded: IpcActionOutputPayload = postcard::from_bytes(&encoded)
    .expect("IpcActionOutputPayload roundtrip must succeed");
```

**Functions**:
- `postcard_roundtrip_with_vec_value` (line 127)
- `postcard_roundtrip_with_bool_value` (line 146)
- `postcard_roundtrip_with_i64_value` (line 164)

---

## 2. `expect()` REPLACEMENT PLAN

152 `expect()` calls exist in `impl_tests.rs` test infrastructure. These are ACCEPTABLE when they wrap **test setup** operations (socket binding, client connections, frame writes) that are known-correct by construction. 152 is excessive; target ≤20 by extracting setup into helper functions.

**Acceptable `expect()`** (in test infra only):
- `IpcServer::bind` — test server setup (≤10 total across all tests)
- `UnixStream::connect` — client socket setup (≤10 total)
- `client.write_all` / `client.flush` — test stimulus (≤10 total)

**Unacceptable `expect()`** — replace with `unwrap()` or proper error propagation:
- All `postcard::to_allocvec(...).expect(...)` → use `?` or `unwrap()`
- All `postcard::from_bytes(...).expect(...)` → use `?` or `unwrap()`

**Refactoring approach**: Extract common client/server setup into helper functions:
```rust
fn with_test_server(path: &Path) -> (IpcServer, Runtime) { ... }
fn with_connected_client(path: &Path) -> UnixStream { ... }
fn send_health_frame(client: &mut UnixStream) { ... }
fn read_response_header(client: &mut UnixStream) -> IpcFrameHeader { ... }
```

This reduces 152 `expect()` calls to ~15 in the test infra layer.

---

## 3. TESTING TROPHY ALLOCATION

### Target distribution (currently ~60% integration, ~30% unit, ~5% e2e, ~5% static)
After fixes, maintain ~60% integration / ~30% unit / ~5% e2e / ~5% static.

### Layer assignments

| Layer | Files | Target Coverage |
|-------|-------|-----------------|
| **Static** | `clippy`, `cargo-deny`, `rustfmt` | 100% — free |
| **Unit** | `ids.rs`, `metrics.rs`, `action_output.rs`, `error.rs`, `frame.rs`, `bounded.rs`, `codec.rs` | 85%+ |
| **Integration** | `handlers.rs`, `impl_tests.rs`, `trace.rs`, `client.rs`, `dispatch.rs` | 70%+ handlers/dispatch/client |
| **E2E** | Full IPC socket roundtrip with real runtime | 5% — critical paths only |

---

## 4. BDD SCENARIOS

### 4.1 Payload Roundtrip (codec)

**Behavior: IpcPayload encodes and decodes losslessly**
Given: A valid `IpcPayload` variant with owned bytes
When: It is encoded with postcard and decoded back
Then: The decoded payload equals the original

```
fn ipc_payload_roundtrips_cancel_run()
fn ipc_payload_roundtrips_drain_trace()
fn ipc_payload_roundtrips_shutdown()
fn ipc_payload_roundtrips_list_events()
fn ipc_payload_roundtrips_inspect_run()
fn ipc_payload_roundtrips_answer_ask()
fn ipc_payload_roundtrips_complete_action()
fn ipc_payload_roundtrips_fail_action()
fn ipc_payload_roundtrips_get_metrics()
fn ipc_payload_roundtrips_list_runs()
fn ipc_payload_roundtrips_submit_run()
fn ipc_payload_roundtrips_submit_run_inline()
```

**Behavior: Invalid bytes produce PayloadError with diagnostic 0x300D**
Given: garbage bytes (`[0xFF, 0xFE, 0xFD, 0xFC]`)
When: passed to `decode_payload::<IpcPayload>`
Then: returns `Err(IpcResponse::PayloadError { diagnostic: 0x300D, message: non_empty })`

**Behavior: Empty bytes produce PayloadError**
Given: empty slice `&[]`
When: passed to `decode_payload::<IpcPayload>`
Then: returns `Err(IpcResponse::PayloadError { .. })`

---

### 4.2 IPC Frame Header

**Behavior: Valid frame header encodes and decodes**
Given: `IpcFrameHeader::new(command, reserved, correlation, payload_len)`
When: encoded to bytes then decoded
Then: all fields match original

**Behavior: Invalid magic produces `InvalidMagic` error**
Given: header bytes with magic `0x00000000` instead of `IPC_MAGIC (0x56424C54)`
When: decoded via `IpcFrameHeader::decode`
Then: returns `Err(IpcError::InvalidMagic { actual: 0x0 })`

**Behavior: Unsupported version produces `UnsupportedVersion` error**
Given: header bytes with version `99`
When: decoded via `IpcFrameHeader::decode`
Then: returns `Err(IpcError::UnsupportedVersion { actual: 99 })`

**Behavior: Non-zero reserved field produces `ReservedNonZero` error**
Given: header bytes with reserved field set to `1`
When: decoded via `IpcFrameHeader::decode`
Then: returns `Err(IpcError::ReservedNonZero { actual: 1 })`

---

### 4.3 IPC Response Types

**Behavior: `typed_events_response` returns Events for valid input**
Given: `events: Vec<TraceEvent>` and `from_sequence: u64`
When: `typed_events_response(&events, from_sequence)` is called
Then: returns `IpcResponse::Events { events: filtered_and_indexed }`

**Behavior: `typed_events_response` filters by from_sequence**
Given: events at indices 0, 1, 2 and `from_sequence = 1`
When: `typed_events_response` is called
Then: result contains only events at indices 1, 2

**Behavior: `typed_events_response` returns empty for empty events**
Given: empty `Vec<TraceEvent>` and any `from_sequence`
When: `typed_events_response` is called
Then: returns `IpcResponse::Events { events: empty }`

**Behavior: `count_response_trace` returns `CountOutOfRange` when u32 overflow**
Given: `count = u32::MAX as usize + 1`
When: `count_response_trace(count)` is called
Then: returns `IpcResponse::CountOutOfRange { actual: u32::MAX as usize + 1, limit: u32::MAX }`

---

### 4.4 Metrics Roundtrip

**Behavior: RuntimeMetrics encodes and decodes losslessly**
Given: `RuntimeMetrics` with populated shards, journal, ipc, totals
When: encoded with postcard and decoded back
Then: decoded equals original

**Behavior: ShardMetrics encodes and decodes losslessly**
Given: `ShardMetrics` with max values
When: roundtripped through postcard
Then: all fields match (including `u32::MAX`, `u64::MAX`)

**Behavior: RuntimeMetrics with empty shards decodes correctly**
Given: `RuntimeMetrics { shards: Vec::new(), .. }`
When: roundtripped through postcard
Then: `decoded.shards.is_empty()` is true

---

### 4.5 ID Types

**Behavior: AskTicketId encodes and decodes losslessly**
Given: `AskTicketId::from_wire(0x1234_5678_9ABC_DEF0)`
When: roundtripped through postcard
Then: `decoded.wire_value() == original.wire_value()`

**Behavior: AskTicketId ordering is consistent with wire values**
Given: `a = AskTicketId::from_wire(1)`, `b = AskTicketId::from_wire(2)`
When: compared with `Ord`
Then: `a < b`

**Behavior: AskTicketId boundary values roundtrip correctly**
Given: wire values `[0, u64::MAX, 0x0000_0000_0000_FFFF]`
When: each is roundtripped through postcard
Then: all decoded wire values match originals

**Behavior: ActionTicketId hash consistency**
Given: `a = ActionTicketId::from_wire(42)`, `b = ActionTicketId::from_wire(42)`
When: inserted into `HashSet`
Then: `set.len() == 1`

---

### 4.6 Action Output Payload

**Behavior: IpcActionOutputPayload with Vec<u8> roundtrips**
Given: `IpcActionOutputPayload { output_slot: SlotIdx::new(0), value: Vec([1,2,3]), taint: Clean }`
When: roundtripped through postcard
Then: `decoded.output_slot == original.output_slot && decoded.value == original.value`

**Behavior: IpcActionOutputPayload with Bool false roundtrips**
Given: `IpcActionOutputPayload { value: Bool(false), .. }`
When: roundtripped through postcard
Then: `decoded.value == SlotValue::Bool(false)`

**Behavior: IpcActionOutputPayload with i64 negative roundtrips**
Given: `IpcActionOutputPayload { value: I64(-100), taint: DerivedFromSecret }`
When: roundtripped through postcard
Then: `decoded.value == SlotValue::I64(-100) && decoded.taint == DerivedFromSecret`

---

### 4.7 Error Diagnostics

**Behavior: Every IpcError variant maps to a unique diagnostic code**
Given: each `IpcError` variant
When: `diagnostic_code()` is called
Then: returns the unique `DiagnosticCode` defined in `error.rs`

**Behavior: IpcError maps to correct runtime_code or None**
Given: each `IpcError` variant
When: `runtime_code()` is called
Then: returns `Some(code)` for frame/queue errors, `None` for encode-only failures

---

## 5. PROPTEST INVARIANTS

### 5.1 ID types — `AskTicketId` and `ActionTicketId`

**Invariant**: `from_wire(wire_value).wire_value() == wire_value` for all `u64`
```rust
proptest! {
    #[test]
    fn ask_ticket_id_roundtrip_is_identity(wire in any::<u64>()) {
        let id = AskTicketId::from_wire(wire);
        prop_assert_eq!(id.wire_value(), wire);
    }

    #[test]
    fn action_ticket_id_roundtrip_is_identity(wire in any::<u64>()) {
        let id = ActionTicketId::from_wire(wire);
        prop_assert_eq!(id.wire_value(), wire);
    }
}
```

**Invariant**: `a == b` iff `a.wire_value() == b.wire_value()`
```rust
proptest! {
    #[test]
    fn ask_ticket_id_equality_depends_on_wire(a in any::<u64>(), b in any::<u64>()) {
        prop_assert_eq!(AskTicketId::from_wire(a) == AskTicketId::from_wire(b), a == b);
    }
}
```

**Invariant**: `Ord` ordering matches `u64` ordering
```rust
proptest! {
    #[test]
    fn ask_ticket_id_ord_matches_wire_ord(a in any::<u64>(), b in any::<u64>()) {
        let ord = AskTicketId::from_wire(a).cmp(&AskTicketId::from_wire(b));
        let expected = a.cmp(&b);
        prop_assert_eq!(ord, expected);
    }
}
```

---

### 5.2 Metrics types

**Invariant**: All metrics types roundtrip through postcard for any valid values
```rust
proptest! {
    #[test]
    fn runtime_metrics_roundtrip_is_identity(
        shard_id in 0u32..10,
        active_runs in 0u32..100,
        writer_queue_depth in any::<u64>(),
        total_events in any::<u64>(),
        connected_clients in 0u32..50,
        commands in any::<u64>(),
        runs_active in any::<u64>(),
    ) {
        let metrics = RuntimeMetrics { /* ... */ };
        let encoded = postcard::to_allocvec(&metrics).unwrap();
        let decoded: RuntimeMetrics = postcard::from_bytes(&encoded).unwrap();
        prop_assert_eq!(decoded, metrics);
    }
}
```

---

### 5.3 Frame header

**Invariant**: `encode().decode() == original` for all valid header combinations
```rust
proptest! {
    #[test]
    fn frame_header_roundtrip_is_identity(
        command in any::<u16>(),
        reserved in 0u16..=0,  // must be 0
        correlation in any::<u64>(),
        payload_len in 0u32..1_000_000,
    ) {
        let header = IpcFrameHeader::new(
            IpcCommand::try_from(command).unwrap_or(IpcCommand::Health),
            reserved,
            correlation,
            payload_len,
        );
        let encoded = header.encode().unwrap();
        let decoded = IpcFrameHeader::decode(&encoded).unwrap();
        prop_assert_eq!(decoded.command(), header.command());
        prop_assert_eq!(decoded.correlation(), header.correlation());
        prop_assert_eq!(decoded.payload_len(), header.payload_len());
    }
}
```

---

### 5.4 Trace events filtering

**Invariant**: `typed_events_response(events, from_sequence)` always returns events with `sequence >= from_sequence`
```rust
proptest! {
    #[test]
    fn typed_events_response_respects_from_sequence(
        events: Vec<TraceEvent>,
        from_sequence in any::<u64>(),
    ) {
        let response = typed_events_response(&events, from_sequence);
        match response {
            IpcResponse::Events { events: result } => {
                for evt in &result {
                    prop_assert!(evt.sequence >= from_sequence);
                }
            }
            IpcResponse::CountOutOfRange { .. } => {
                // valid when from_sequence overflows
            }
            other => panic!("unexpected response: {:?}", other),
        }
    }
}
```

---

## 6. FUZZ TARGETS

### 6.1 Payload decoder (`handlers.rs`)

**Target**: `decode_payload::<IpcPayload>(bytes)`
**Risk**: HIGH — untrusted bytes → typed deserialization
**Corpus seeds**: Valid encoded `IpcPayload` variants from known test fixtures
**Approach**:  
- Generate random bytes → feed to `decode_payload` → verify no panic, no alloc bomb
- Validate that error responses have non-empty messages
```rust
#[derive(Arbitrary)]
struct FuzzPayloadInput<'a> {
    bytes: &'a [u8],
}

fn fuzz_decode_payload(input: FuzzPayloadInput<'_>) {
    let result = decode_payload::<crate::IpcPayload>(input.bytes);
    match result {
        Ok(payload) => { /* valid, check invariants */ }
        Err(IpcResponse::PayloadError { message, .. }) => {
            assert!(!message.is_empty(), "error message must be non-empty");
        }
        Err(_) => { /* other error variants are valid */ }
    }
}
```

### 6.2 Frame header decoder (`frame.rs`)

**Target**: `IpcFrameHeader::decode(bytes)`
**Risk**: HIGH — fixed-width header parsing from untrusted bytes
**Corpus seeds**: Valid frame headers in hex
**Approach**: Random 16-byte sequences → feed to `decode` → verify no panic
```rust
fn fuzz_frame_header_decode(bytes: &[u8]) {
    if bytes.len() == IPC_HEADER_LEN {
        let _ = IpcFrameHeader::decode(bytes);
    }
}
```

### 6.3 Metrics postcard roundtrip (`metrics.rs`)

**Target**: `postcard::from_bytes::<RuntimeMetrics>(bytes)`
**Risk**: MEDIUM — metrics are internal but could have malformed values
```rust
fn fuzz_metrics_decode(bytes: &[u8]) {
    if let Ok(metrics) = postcard::from_bytes::<RuntimeMetrics>(bytes) {
        // validate metric sanity: non-negative, reasonable bounds
        assert!(metrics.ipc.connected_clients <= 10_000);
    }
}
```

---

## 7. KANI HARNESSES

### 7.1 Frame header bounds

**Property**: `IpcFrameHeader::new` with valid inputs never panics
**Bound**: `command: u16`, `reserved: u16` (must be 0), `correlation: u64`, `payload_len: u32`
```rust
#[kani::proof]
fn header_construction_is_safe() {
    let command = kani::any::<u16>();
    let reserved = kani::any::<u16>();
    let correlation = kani::any::<u64>();
    let payload_len = kani::any::<u32>();

    // If reserved is 0, header construction succeeds
    if reserved == 0 {
        let header = IpcFrameHeader::new(command, reserved, correlation, payload_len);
        kani::assert(header.is_valid(), "header must be valid");
    }
}
```

### 7.2 usize/u32 conversion in error mapping

**Property**: `u32_to_usize` returns correct value or `PayloadLengthOutOfRange` error
```rust
#[kani::proof]
fn u32_to_usize_is_correct_or_error() {
    let value = kani::any::<u32>();
    match usize::try_from(value) {
        Ok(v) => {
            let result = u32_to_usize(value);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), v);
        }
        Err(_) => {
            let result = u32_to_usize(value);
            assert!(result.is_err());
        }
    }
}
```

---

## 8. MUTATION TESTING CHECKPOINTS

**Target**: ≥90% kill rate

| Mutation | Kill Mechanism | File |
|----------|---------------|------|
| Change `wire_value()` return to `0` in `AskTicketId` | `ask_ticket_id_roundtrip_is_identity` proptest | `ids.rs` |
| Change `wire_value()` return to `0` in `ActionTicketId` | `action_ticket_id_roundtrip_is_identity` proptest | `ids.rs` |
| Swap `u64::try_from(index)` to `Ok(0)` in `typed_events_response` | `typed_events_response_preserves_event_kind_mapping` | `trace.rs` |
| Change `diagnostic_code()` match arm | `error_diagnostic_codes_are_unique` | `error.rs` |
| Change `frame.rs` `MAGIC` constant | `frame_header_decode_rejects_invalid_magic` | `frame.rs` |
| Change `bounded.rs` max bounds | `bounded_decode_rejects_oversized_payload` | `bounded.rs` |
| Flip `Ok`/`Err` in `decode_payload` | `decode_payload_returns_error_for_garbage_bytes` | `handlers.rs` |
| Change payload length check comparison | `submit_run_oversized_input_survives_decode_for_handler_check` | `handlers.rs` |

---

## 9. COMBINATORIAL COVERAGE MATRIX

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| `IpcPayload::CancelRun` roundtrip | valid `RunId(42)` | `Ok(CancelRun { run_id: 42 })` | unit |
| `IpcPayload::Health` roundtrip | `Health` unit variant | `Ok(Health)` | unit |
| garbage bytes decode | `[0xFF; 4]` | `Err(PayloadError { diagnostic: 0x300D })` | unit |
| empty bytes decode | `&[]` | `Err(PayloadError { .. })` | unit |
| `AskTicketId` identity | any `u64` wire value | `wire_value() == original` | proptest |
| `ActionTicketId` identity | any `u64` wire value | `wire_value() == original` | proptest |
| `RuntimeMetrics` roundtrip | populated struct | `decoded == original` | unit |
| `RuntimeMetrics` empty shards | `Vec::new()` | `shards.is_empty()` | unit |
| `typed_events_response` filter | 3 events, from_seq=1 | 2 events with seq≥1 | unit |
| `typed_events_response` empty | `Vec::new()` | `Events { events: [] }` | unit |
| `count_response_trace` overflow | `u32::MAX as usize + 1` | `CountOutOfRange` | unit |
| frame header invalid magic | magic=`0x0` | `Err(InvalidMagic)` | unit |
| frame header bad version | version=`99` | `Err(UnsupportedVersion)` | unit |
| frame header reserved≠0 | reserved=`1` | `Err(ReservedNonZero)` | unit |
| dispatch `Health` command | valid header+payload | `IpcResponse::Healthy` | integration |
| dispatch `Shutdown` command | valid header+payload | `IpcResponse::ShutdownAck` | integration |
| dispatch unknown command | command_id=`9999` | `IpcResponse::FrameError` | integration |
| client connect then disconnect | server socket path | no panic, clean close | integration |
| pipelined frames | 3 frames sent before poll | each processed in order | integration |
| `IpcError::PayloadTooLarge` | actual=`1_000_000`, limit=`100_000` | `PayloadTooLarge { actual, limit }` | unit |

---

## 10. EXPECT() FIXES (152 total)

**Location**: `impl_tests.rs` — test infrastructure only

The 152 `expect()` calls are in test setup code. Replace with extracted helper functions to reduce to ≤20 direct `expect()` calls.

### Helper extraction targets:

```rust
// Instead of 20+ .expect("bind should succeed") in tests:
fn spawn_test_server(name: &str) -> (IpcServer, Runtime, PathBuf) {
    let path = temp_socket_path(name);
    let server = IpcServer::bind(&path).expect("test server bind");
    let runtime = make_runtime();
    (server, runtime, path)
}

// Instead of 20+ .expect("client connect") in tests:
fn connect_client(path: &Path) -> UnixStream {
    UnixStream::connect(path).expect("test client connect")
}

// Instead of 10+ .expect("write frame") in tests:
fn write_frame(client: &mut UnixStream, frame: &[u8]) {
    client.write_all(frame).expect("write test frame");
    client.flush().expect("flush test frame");
}
```

After helper extraction: ~15 `expect()` calls remain (in test infra that must succeed).

---

## 11. COVERAGE RAISE PLAN

### handlers.rs (44% → 70%)

**Gap**: Uncovered branches in handler response matching
**Action**: Add integration tests for all `IpcResponse` variants returned from handlers

```
fn handler_returns_health_for_health_command()
fn handler_returns_shutdown_ack_for_shutdown_command()
fn handler_returns_error_for_unknown_command()
fn handler_returns_workflow_graph_for_valid_digest()
fn handler_returns_taint_report_for_valid_digest()
fn handler_returns_submit_run_acknowledged()
```

### dispatch.rs (23% → 50%)

**Gap**: `dispatch_command_with_resolver` branches for all `IpcCommand` variants
**Action**: Test all 15 command dispatch paths directly (not through full server)

```
fn dispatch_health_command_returns_healthy()
fn dispatch_submit_run_command_returns_submitted()
fn dispatch_submit_run_inline_command_returns_submitted()
fn dispatch_cancel_run_command_returns_cancelled_or_error()
fn dispatch_inspect_run_command_returns_inspection()
fn dispatch_list_events_command_returns_events_or_error()
fn dispatch_answer_ask_command_returns_answered_or_error()
fn dispatch_complete_action_command_returns_completed_or_error()
fn dispatch_fail_action_command_returns_failed_or_error()
fn dispatch_drain_trace_command_returns_traces()
fn dispatch_list_runs_command_returns_runs()
fn dispatch_get_metrics_command_returns_metrics()
fn dispatch_verify_workflow_command_returns_verification()
fn dispatch_get_workflow_graph_command_returns_graph()
fn dispatch_get_taint_report_command_returns_taint_report()
```

### client.rs (48% → 70%)

**Gap**: `recv_response` error paths, `send_raw` error paths
**Action**: Add unit tests for client error handling with injected frame errors

```
fn client_recv_response_header_handles_would_block()
fn client_recv_response_header_handles_eof()
fn client_recv_response_payload_handles_truncated_frame()
fn client_send_raw_handles_write_error()
fn client_health_sends_correct_frame()
fn client_shutdown_sends_correct_frame()
fn client_list_runs_sends_correct_payload()
```

---

## 12. SUMMARY METRICS TARGET

| Metric | Current | Target |
|--------|---------|--------|
| `assert!(false, ...)` count | 76 | 0 |
| `expect()` in test code | 152 | ≤20 |
| handlers.rs coverage | 44% | ≥70% |
| dispatch.rs coverage | 23% | ≥50% |
| client.rs coverage | 48% | ≥70% |
| proptest invariants | 0 | 6 |
| fuzz targets | 0 | 3 |
| Kani harnesses | 0 | 2 |
| mutation kill rate | unknown | ≥90% |
| density multiplier | 1.81x | ≥5x |

---

## 13. EXECUTION ORDER

1. **Phase 1**: Fix all 76 `assert!(false, ...)` tautologies (2 files per pass)
2. **Phase 2**: Extract test infra helpers, reduce `expect()` count
3. **Phase 3**: Add proptest invariants for `ids.rs` and `metrics.rs`
4. **Phase 4**: Add fuzz targets for payload decode and frame decode
5. **Phase 5**: Add Kani harnesses for `u32_to_usize` and frame header
6. **Phase 6**: Raise coverage in handlers.rs, dispatch.rs, client.rs
7. **Phase 7**: Run mutation testing, verify ≥90% kill rate
8. **Phase 8**: Final `cargo test -p vb_ipc` and `cargo clippy -p vb_ipc`
