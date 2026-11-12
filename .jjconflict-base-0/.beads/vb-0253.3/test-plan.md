# Test Plan: vb-0253.3 — IPC Bridge Bounded Channel Change

## Summary
- **Bead**: vb-0253.3 — ui: Bound IPC bridge channels with backpressure
- **Scope**: `crates/vb_ui/src/ipc_bridge.rs`
- **Behaviors identified**: 8
- **Trophy allocation**: 2 unit / 4 integration / 0 e2e / 2 static
- **Proptest invariants**: 0 (no multi-input pure functions in scope)
- **Fuzz targets**: 0 (no parsing boundaries in scope)
- **Kani harnesses**: 0 (no unsafe code; `sync_channel` is stdlib trusted)

---

## 1. Behavior Inventory

| # | Subject | Behavior |
|---|---------|----------|
| B1 | `IpcBridge::new()` | Creates both request and reply channels as bounded `sync_channel` with capacity 16 |
| B2 | `IpcBridge::send()` | Returns `Ok(())` when the bounded request channel has capacity available |
| B3 | `IpcBridge::send()` | Returns `Err(String)` containing `"full"` when the bounded request channel is at capacity |
| B4 | `IpcBridge::send()` | Returns `Err(String)` containing `"disconnected"` when the request channel sender is dropped |
| B5 | `IpcBridge::poll()` | Drains all available replies via `try_recv` without blocking the UI thread |
| B6 | `IpcBridge::poll()` | Updates `connected` field on `Connected`/`Disconnected`/`ConnectionFailed` replies |
| B7 | `IpcBridge::is_connected()` | Returns `true` after `Connected` reply; `false` after `Disconnected`/`ConnectionFailed` |
| B8 | Background thread | Uses `recv_timeout(100ms)` loop — unchanged from unbounded channel behavior |

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Static Analysis** | 2 | VB0253-CLIPPY-001 (cargo clippy), VB0253-LINT-001 (`#![forbid(unsafe_code)]`) — compile-time gates, zero runtime cost |
| **Unit / Calc** | 2 | Pure helper functions: `reply_from_response`, `reply_from_submit`, `reply_from_answer`, `reply_from_drain_trace`, `next_correlation` — deterministic, no I/O |
| **Integration** | 4 | `IpcBridge` public API behaviors (B1–B8) — real background thread, real `crossbeam_channel`, actual socket I/O avoided by connecting to nonexistent path |
| **E2E** | 0 | No CLI/API surface for `IpcBridge`; UI integration out of scope per contract non-goals |

**Rationale**: Bounded channel backpressure is fundamentally an integration-level concern (producer-consumer timing with real threading). Unit coverage is limited to pure helpers. No fuzzing needed (no parsing boundaries in `IpcBridge`). No Kani needed (no unsafe code).

---

## 3. BDD Scenarios

### B1: `IpcBridge::new()` creates bounded channels

**Scenario**: `bridge_new_creates_channels_and_thread`
- Given: No bridge exists
- When: `IpcBridge::new()` is called
- Then: `poll()` returns empty (channel initialized but background thread has not yet replied)
- And: `is_connected()` returns `false`
- And: Thread handle is `Some` (thread spawned)

**Layer**: integration

---

### B2: `send()` returns Ok when channel has capacity

**Scenario**: `bridge_send_without_connect_returns_not_connected_error` (existing)
- Given: `IpcBridge::new()` has been called
- When: `send(Health)` is called (before any connection)
- Then: `send()` returns `Ok(())` (request queued in bounded channel)
- And: The background thread replies with `Error("Not connected")` within 100ms

**Layer**: integration

**Gap**: No explicit test verifies `Ok(())` is returned when channel has capacity (only the "disconnected" path is tested). A dedicated "happy path" test should exist.

---

### B3: `send()` returns backpressure error when channel is full

**Scenario**: `bridge_send_on_full_returns_error`
- Given: `IpcBridge::new()` with background thread processing requests at 100ms intervals
- When: `CHANNEL_CAPACITY + 1` requests are sent in rapid succession (before thread drains)
- Then: The 17th `send()` returns `Err(String)`
- And: The error string contains `"full"` (POST-003) or matches `TrySendError::Full` variant

**Layer**: integration

**⚠️ CRITICAL ISSUE FOUND**: The existing test at lines 908–933 asserts `err_msg.contains("channel full")` but `crossbeam_channel::TrySendError::Full` formats as `Full(Health)` — it does NOT contain the phrase "channel full". This assertion will **always fail** even when the bounded channel change is correctly implemented.

**Required fix**: Change the assertion from `err_msg.contains("channel full")` to check for `TrySendError::Full` variant, or update the error-mapping logic to produce "channel full" text explicitly.

**Error variant — disconnected**:
- Given: Background thread has exited (sender half dropped)
- When: `send()` is called on the now-disconnected channel
- Then: Returns `Err(String)` containing `"disconnected"` (POST-004)

**Layer**: integration

---

### B4: `send()` returns disconnected error when sender dropped

**Scenario**: `bridge_send_without_connect_returns_not_connected_error`
- Given: `IpcBridge::new()` with background thread that dies (simulated via `IpcRequest::Connect` to nonexistent socket causing thread exit)
- When: After thread death, `send(Health)` is called
- Then: Returns `Err(String)` containing `"disconnected"` or `"send failed"` from `TrySendError::Disconnected`

**Layer**: integration

---

### B5: `poll()` drains replies without blocking

**Scenario**: `bridge_new_creates_channels_and_thread`
- Given: `IpcBridge::new()`
- When: `poll()` is called immediately
- Then: Returns empty `Vec` (no replies pending; `try_recv` was used, not blocking `recv`)

**Layer**: integration

**Invariant**: `poll()` never blocks — confirmed by `try_recv` usage in source (line 199).

---

### B6: `poll()` updates `connected` field

**Scenario**: `bridge_connect_to_nonexistent_socket_fails`
- Given: `IpcBridge::new()`
- When: `IpcRequest::Connect { socket_path }` to nonexistent path is sent, and `ConnectionFailed` reply is received via `poll()`
- Then: `is_connected()` returns `false` after polling `ConnectionFailed`

**Layer**: integration

---

### B7: `is_connected()` reflects last connection state

**Scenario**: `bridge_connect_to_nonexistent_socket_fails`
- Given: `IpcBridge::new()`
- When: `poll()` receives `ConnectionFailed` reply
- Then: `is_connected()` returns `false`

**Scenario**: (No existing test for `Connected` state transitioning to `true`)
- Given: Bridge is connected
- When: `poll()` receives `Connected` reply
- Then: `is_connected()` returns `true`

**Layer**: integration

**Gap**: No test verifies `is_connected()` returns `true` after a successful connection. A `Connect`-to-real-socket test is not feasible without a running server, but a mock or the existing `bridge_connect_to_nonexistent_socket_fails` path does not cover the `true` case.

---

### B8: Background thread uses 100ms recv_timeout

**Scenario**: (Covered by `bridge_connect_to_nonexistent_socket_fails`)
- Given: `IpcBridge::new()`
- When: `Connect` to nonexistent socket is sent
- Then: `poll()` receives `ConnectionFailed` within 500ms (thread loop processes at 100ms intervals)

**Layer**: integration

---

## 4. Proptest Invariants

**N/A** — No pure multi-input functions in scope. All public API methods involve threading/I/O. Helper functions (`reply_from_*`, `next_correlation`) are pure but single-output; their outputs are exhaustively tested via unit tests already present in the file.

---

## 5. Fuzz Targets

**N/A** — No parsing or deserialization boundaries in `IpcBridge`. All inputs (`IpcRequest`, `IpcReply`) are generated internally or come from a trusted in-process workflow engine.

---

## 6. Kani Harnesses

**N/A** — Per VB0253-WAIVER-KANI-001 and VB0253-WAIVER-VERUS-001:
- `ipc_bridge.rs` contains no `unsafe` blocks (`#![forbid(unsafe_code)]` on line 1)
- `crossbeam_channel::sync_channel` is stdlib-adjacent trusted code
- No refinement types, ghost state, or arithmetic overflow risks in scope
- SPSC (single-producer single-consumer) mpsc has no concurrent interleavings to explore

---

## 7. Mutation Checkpoints

| Mutation | Must be caught by | Threshold |
|----------|-------------------|-----------|
| `bounded()` → `channel()` (unbounded) | `bridge_send_on_full_returns_error` would always pass (never Full error) | 90% |
| `try_send()` → `send()` (blocking) | `bridge_send_on_full_returns_error` would hang forever on full channel | 90% |
| `try_recv()` → `recv()` in `poll()` | UI thread would block on `poll()`, breaking render loop contract | 90% |
| `recv_timeout(100ms)` → `recv()` (blocking) | Background thread would block forever on empty channel | 90% |
| `CHANNEL_CAPACITY = 16` → `CHANNEL_CAPACITY = 0` | `bridge_send_on_full_returns_error` would fail on 1st send | 90% |
| Error message `"{e}"` → `"fixed string"` | `bridge_send_without_connect_returns_not_connected_error` would lose error specificity | 90% |

---

## 8. Combinatorial Coverage Matrix

### `IpcBridge::send()` — POST-002 / POST-003 / POST-004

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| Happy path | `Health` request, channel has capacity | `Ok(())` | integration |
| Channel full | 17th rapid `Health` request, channel at capacity 16 | `Err(String)` containing `"full"` | integration |
| Sender disconnected | `send()` after background thread exits | `Err(String)` containing `"disconnected"` or `"send failed"` | integration |
| `Connect` then `Health` | Request while disconnected | `Ok(())` queued; error reply via `poll()` | integration |

### `IpcBridge::poll()` — POST-005 / POST-006 / INV-002

| Scenario | Input | Expected Output | Layer |
|----------|-------|-----------------|-------|
| Empty poll | No replies pending | `Vec::new()` | integration |
| Drains one reply | `Connected` reply pending | `vec![Connected]`, `is_connected() = true` | integration |
| Drains disconnected | `ConnectionFailed` reply pending | `vec![ConnectionFailed(...)]`, `is_connected() = false` | integration |
| Drains multiple | Multiple replies pending | `Vec` with all replies, `connected` last | integration |

### Helper functions — unit

| Function | Input | Expected | Layer |
|----------|-------|---------|-------|
| `reply_from_response(Healthy)` | `IpcResponse::Healthy` | `IpcReply::Healthy` | unit |
| `reply_from_response(RuntimeError{"boom"})` | `IpcResponse::RuntimeError` | `IpcReply::Error("boom")` | unit |
| `reply_from_submit(AcceptedRun{7})` | `IpcResponse::AcceptedRun{7}` | `IpcReply::RunAccepted(7)` | unit |
| `reply_from_answer(BadRequest)` | `IpcResponse::BadRequest` | `IpcReply::Error` | unit |
| `reply_from_drain_trace(TraceCount{42})` | `IpcResponse::TraceCount{42}` | `IpcReply::TraceCount(42)` | unit |
| `next_correlation(0)` | `u64 = 0` | `1` | unit |
| `next_correlation(u64::MAX)` | `u64 = MAX` | `0` (wrap) | unit |

---

## 9. Open Questions

### O1: Error string format mismatch (BLOCKING)
**Q**: Does `crossbeam_channel::TrySendError::Full` contain `"channel full"` in its `Display` implementation?
**Evidence**: The test at line 930 asserts `err_msg.contains("channel full")`. `crossbeam_channel` formats `TrySendError::Full` as `Full(...)`, NOT `"channel full"`. This test will **always fail** even when bounded channels work correctly.
**Impact**: VB0253-TEST-002 and VB0253-TEST-007 cannot pass as written.
**Resolution needed**: Either (a) change error mapping in `send()` to explicitly match the contract's `"channel full"` string, or (b) update the test to assert on `TrySendError::Full` variant instead of string content.

### O2: Build blockers outside scope
**Q**: `vb_ui` crate fails to compile due to pre-existing errors in `app_state.rs`, `graph_builder.rs`, `registry/mod.rs`, and other files (non-exhaustive patterns, type mismatches).
**Impact**: Cannot run `cargo test -p vb_ui --lib` to execute VB0253-TEST-006 (all 24 existing tests pass).
**Resolution**: These must be fixed separately per VB0253-COMPILE-001 assumption: "Workspace build errors in app_state.rs, node_mapping.rs fixed separately".

### O3: Happy-path `send()` coverage
**Q**: Is there a test that explicitly asserts `send()` returns `Ok(())` when the channel has capacity (not just that it doesn't error)?
**Evidence**: No such test exists. `bridge_send_without_connect_returns_not_connected_error` only asserts `is_ok()` but doesn't verify the specific case where capacity is available.
**Resolution**: Add explicit test `bridge_send_returns_ok_when_channel_has_capacity`.

### O4: `is_connected()` true path
**Q**: Is there a test that verifies `is_connected()` returns `true` after a `Connected` reply?
**Evidence**: `bridge_connect_to_nonexistent_socket_fails` only tests the failure path.
**Resolution**: Add test or document that a successful connect requires a live server (integration test not achievable in unit-only scope).

---

## 10. Verification Evidence Requirements

| Proof ID | Command | Expected Evidence | Status |
|----------|---------|-------------------|--------|
| VB0253-COMPILE-001 | `cargo build -p vb_ui --lib` | ipc_bridge.rs compiles with `bounded` + `sync_channel` | ⚠️ BLOCKED by other files |
| VB0253-COMPILE-002 | `cargo build -p vb_ui --lib` | `const CHANNEL_CAPACITY: usize = 16` assertion passes | ⚠️ BLOCKED |
| VB0253-TEST-001 | `cargo test ipc_bridge::tests::bridge_new_creates_channels_and_thread` | Test passes; `poll().is_empty()` after `new()` | ⚠️ BLOCKED |
| VB0253-TEST-002 | `cargo test ipc_bridge::tests::bridge_send_on_full_returns_error` | Test passes; error contains `"full"` | ⚠️ BLOCKED + FLAWED |
| VB0253-TEST-003 | `cargo test ipc_bridge::tests::bridge_send_without_connect_returns_not_connected_error` | Test passes | ⚠️ BLOCKED |
| VB0253-TEST-004 | Same as TEST-001 | `poll().is_empty()` proves `try_recv` used | ⚠️ BLOCKED |
| VB0253-TEST-005 | `cargo test ipc_bridge::tests::bridge_connect_to_nonexistent_socket_fails` | `is_connected() = false` after `ConnectionFailed` | ⚠️ BLOCKED |
| VB0253-TEST-006 | `cargo test ipc_bridge::tests` (all) | All 24 tests pass | ⚠️ BLOCKED |
| VB0253-TEST-007 | Same as TEST-002 | Same flaw as TEST-002 | ⚠️ BLOCKED + FLAWED |
| VB0253-CLIPPY-001 | `cargo clippy -p vb_ui --lib --bins --examples -- -D warnings` | No warnings | ⚠️ BLOCKED by build |
| VB0253-LINT-001 | `grep -c '#!\[forbid(unsafe_code)\]' ipc_bridge.rs` | `1` (line 1) | ✅ VERIFIED from source |

---

## 11. Recommendations

1. **Fix O1 immediately**: The `bridge_send_on_full_returns_error` test assertion is incorrect. The `send()` error mapping at line 193 uses `format!("IPC send failed: {e}")` where `{e}` is `TrySendError`. The contract requires `"channel full"` but the actual error format is `"IPC send failed: Full(Health)"`. The error mapping must be updated to produce the contract-specified string, e.g.:

   ```rust
   pub fn send(&self, request: IpcRequest) -> Result<(), String> {
       self.tx.try_send(request).map_err(|e| {
           match e {
               TrySendError::Full(_) => "IPC send failed: channel full".to_string(),
               TrySendError::Disconnected(_) => "IPC send failed: disconnected".to_string(),
           }
       })
   }
   ```

2. **Fix O2 by building vb_ui in isolation or using `cargo check --lib`** to verify ipc_bridge.rs specifically compiles while other files are addressed separately.

3. **Add missing tests**: `bridge_send_returns_ok_when_channel_has_capacity` and verify `is_connected()` `true` path documentation.

4. **Do not mutate**: The `bridge_send_on_full_returns_error` test structure (filling channel faster than 100ms drain) is sound; only the assertion string needs correction.
