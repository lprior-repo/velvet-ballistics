# Martin Fowler Test Plan: vb-core-ipc-loom-property

## Happy Path Tests

### MemoryIngress
- `test_memory_ingress_submit_when_not_full_succeeds` — Given channel has capacity, when try_submit is called, then it returns Ok
- `test_memory_ingress_recv_when_not_empty_returns_frame` — Given channel has frames, when try_recv is called, then it returns Some(frame)

### FramePool
- `test_frame_pool_take_allocates_fresh_frame` — Given empty pool, when take is called, then a new frame is allocated
- `test_frame_pool_release_returns_frame_to_pool` — Given a taken frame, when release is called, then the frame is available for reuse
- `test_frame_pool_available_never_exceeds_capacity` — Given capacity=1, when 100 frames are released, then available stays at 1

### IPC Server Client-Map
- `test_ipc_server_client_map_insert_then_remove` — Given a client is inserted, when remove is called, then the client is no longer in the map

### Write Buffer
- `test_write_buffer_fill_then_drain_conserves_bytes` — Given bytes are written to buffer, when drain is called, then exact bytes are removed

## Error Path Tests

### MemoryIngress
- `test_memory_ingress_submit_when_full_returns_full` — Given channel is at capacity, when try_submit is called, then it returns Err(Full)
- `test_memory_ingress_recv_when_disconnected_returns_error` — Given sender is dropped, when try_recv is called, then it returns Err(Disconnected)

### FramePool
- `test_frame_pool_release_silent_drop_at_capacity` — Given pool is at capacity, when release is called, then the frame is silently dropped and no error is returned
- `test_frame_pool_allocation_failure` — Given pool allocation would exceed MAX_POOL_CAPACITY, when new is called, then it returns Err(ResourceLimitExceeded)

### IPC Server
- `test_ipc_server_too_many_clients` — Given MAX_CLIENTS clients are connected, when a new client connects, then it returns Err(TooManyClients)

## Edge Case Tests

### MemoryIngress
- `test_memory_ingress_zero_capacity_panics` — Given capacity=0, when bounded is called, then it panics (crossbeam_channel contract)
- `test_memory_ingress_available_never_exceeds_capacity` — Given capacity=2 and 100 concurrent submits, when all complete, then available <= 2

### FramePool
- `test_frame_pool_capacity_one_never_exceeds_limit` — Given capacity=1, when 100 frames are rapidly released, then available is always 1
- `test_frame_pool_wrong_dimensions_silently_dropped` — Given a frame with mismatched step_count, when released, then it is silently dropped
- `test_frame_pool_reused_frame_has_clean_state` — Given a frame with prior slot values, when reused, then prior state is cleared

### Write Buffer
- `test_write_buffer_partial_drain` — Given buffer has 100 bytes, when 30 bytes are drained, then 30 bytes remain
- `test_write_buffer_drain_exact_bytes_written` — Given buffer has N bytes drained, then Len(buffer) == written - drained

## Contract Verification Tests

### INV-001 (MemoryIngress backpressure)
- `test_precondition_memory_ingress_capacity_positive` — Given capacity > 0, when bounded is called, then it succeeds
- `test_postcondition_memory_ingress_submit_full_error` — Given channel is full, when try_submit fails, then the error is exactly Full
- `test_invariant_memory_ingress_available_never_exceeds_capacity` — After any sequence of submit/recv, available <= capacity

### INV-002 (FramePool capacity)
- `test_precondition_frame_pool_capacity_bounds` — Given capacity=0 or >4096, when new is called, then it returns Err
- `test_postcondition_frame_pool_release_at_capacity_silent_drop` — Given pool at capacity, when release is called, then it returns Ok (no panic) and frame is dropped
- `test_invariant_frame_pool_available_never_exceeds_capacity` — After any sequence of take/release, available() <= capacity()

### INV-003 (IPC client-map)
- `test_invariant_client_map_max_clients_enforced` — After MAX_CLIENTS insertions, the next insertion fails with TooManyClients

### INV-004 (write buffer)
- `test_invariant_write_buffer_byte_conservation` — After any sequence of fill/drain, Len(buffer) == written - drained

## End-to-End Scenarios

### Scenario: MemoryIngress bounded channel backpressure
Given: MemoryIngress with capacity=2, two producers
When: producer A submits 2 frames (fills channel) and producer B tries to submit a 3rd frame
Then: producer B receives Err(Full) and the channel remains at capacity=2

### Scenario: FramePool concurrent take/release
Given: FramePool with capacity=2 and 2 threads concurrently taking and releasing frames
When: both threads rapidly cycle take/release 10 times
Then: available() never exceeds 2 and no frames are lost

### Scenario: IPC slow-client write buffer drain
Given: a client with write_buffer containing 4096 bytes
When: handle_writable is called and writes 1024 bytes
Then: exactly 1024 bytes are drained and 3072 bytes remain in write_buffer

## Given-When-Then Summary

| Scenario | Given | When | Then |
|----------|-------|------|------|
| Ingress full | Channel at capacity | try_submit called | Err(Full) returned |
| Ingress recv empty | Channel empty | try_recv called | Ok(None) returned |
| Frame pool at capacity | Pool has capacity frames | release called | Frame silently dropped |
| Client map max | MAX_CLIENTS connected | new client connects | Err(TooManyClients) |
| Write buffer drain | 100 bytes in buffer | 30 bytes drained | 70 bytes remain |
