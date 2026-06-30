# Martin Fowler Test Plan — vb-0253.3

## Happy Path Tests
- `test_send_returns_ok_when_channel_has_capacity` — send succeeds immediately after bridge creation
- `test_poll_returns_empty_vec_immediately_after_new` — poll does not block and returns no replies
- `test_is_connected_false_immediately_after_new` — bridge starts disconnected
- `test_send_then_poll_delivers_reply` — request/response roundtrip (when connected)

## Error Path Tests
- `test_send_on_full_returns_backpressure_error` — flooding bounded channel beyond capacity produces `Err("IPC send failed: channel full")`
- `test_send_without_connect_returns_not_connected_error` — send to disconnected tx produces error reply (existing test)
- `test_connect_to_nonexistent_socket_produces_connection_failed` — connect to bad socket produces `IpcReply::ConnectionFailed` (existing test)
- `test_health_without_connect_returns_not_connected_error` — Health request without connect returns error (existing test)
- `test_submit_run_without_connect_returns_not_connected_error` — SubmitRun without connect returns error (existing test)
- `test_answer_ask_without_connect_returns_not_connected_error` — AnswerAsk without connect returns error (existing test)
- `test_drain_trace_without_connect_returns_not_connected_error` — DrainTrace without connect returns error (existing test)

## Edge Case Tests
- `test_send_exactly_capacity_requests_all_succeed` — sending exactly `CHANNEL_CAPACITY` requests without polling all succeeds with `Ok`
- `test_send_capacity_plus_one_produces_one_backpressure_error` — sending `CHANNEL_CAPACITY + 1` requests produces exactly one backpressure error
- `test_poll_drains_all_pending_replies_without_blocking` — multiple replies are all drained in one `poll()` call
- `test_correlation_counter_wraps_at_u64_max` — `next_correlation` wraps correctly (existing test)
- `test_correlation_counter_increments_from_zero` — counter starts at 0 and increments (existing test)

## Contract Verification Tests
- `test_new_constructs_bounded_channels` — compile-time verification that `sync_channel` is used, not unbounded `channel`
- `test_send_error_contains_channel_full_when_full` — backpressure error string contains "channel full"
- `test_send_error_contains_disconnected_when_thread_dead` — disconnected error string is preserved
- `test_poll_uses_try_recv_not_recv` — poll implementation uses non-blocking `try_recv` (verified by code inspection + behavior)
- `test_reply_from_response_healthy` — existing test preserved
- `test_reply_from_response_shutting_down` — existing test preserved
- `test_reply_from_response_runtime_error` — existing test preserved
- `test_reply_from_submit_accepted_run` — existing test preserved
- `test_reply_from_answer_accepted_run` — existing test preserved
- `test_reply_from_drain_trace_count` — existing test preserved

## Given-When-Then Scenarios

### Scenario 1: Send request when channel has capacity
**Given**: A newly created `IpcBridge` with bounded request channel at zero depth
**When**: UI thread calls `bridge.send(IpcRequest::Health)`
**Then**: `send()` returns `Ok(())` and the request is queued in the bounded channel

### Scenario 2: Send request when channel is at capacity (backpressure)
**Given**: A bridge whose bounded request channel is full (background thread is slow and not draining)
**When**: UI thread calls `bridge.send(IpcRequest::Health)`
**Then**: `send()` returns `Err(String)` containing "channel full" (backpressure signal to UI)

### Scenario 3: Send request when background thread has died
**Given**: A bridge where the background thread has exited (e.g., socket disconnected fatally)
**When**: UI thread calls `bridge.send(IpcRequest::Health)`
**Then**: `send()` returns `Err(String)` describing the disconnected channel

### Scenario 4: Poll drains all available replies without blocking
**Given**: A connected bridge with multiple pending `IpcReply` values in the reply channel
**When**: UI thread calls `bridge.poll()`
**Then**: All available replies are returned immediately as a `Vec<IpcReply>` without blocking

### Scenario 5: is_connected reflects connection state
**Given**: A bridge in disconnected state
**When**: `poll()` delivers an `IpcReply::Connected` reply
**Then**: `is_connected()` returns `true`

### Scenario 6: is_connected false after connection failure
**Given**: A bridge that was previously connected
**When**: `poll()` delivers an `IpcReply::ConnectionFailed` reply
**Then**: `is_connected()` returns `false`

### Scenario 7: Correlation counter wraps at u64::MAX
**Given**: `next_correlation` has been called until `c == u64::MAX`
**When**: `next_correlation(&mut c)` is called
**Then**: It returns `0` and `c == 0`

### Scenario 8: Health request returns Not connected when disconnected
**Given**: A bridge with no active IPC connection
**When**: `bridge.send(IpcRequest::Health)` is called
**Then**: `poll()` eventually returns `IpcReply::Error(e)` where `e` contains "Not connected"
