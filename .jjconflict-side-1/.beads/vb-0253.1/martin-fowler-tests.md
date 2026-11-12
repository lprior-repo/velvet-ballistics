# Martin Fowler Test Plan - vb-0253.1

## Happy Path Tests
- test_command_queue_accepts_commands_up_to_capacity
- test_dequeue_returns_command_in_fifo_order
- test_command_queue_len_increments_on_enqueue

## Error Path Tests
- test_enqueue_returns_error_when_queue_full
- test_dequeue_returns_none_when_queue_empty
- test_config_rejects_zero_capacity
- test_config_rejects_excessive_capacity

## Edge Case Tests
- test_command_queue_at_exact_capacity_accepts_one_more_then_rejects
- test_enqueue_after_partial_dequeue_succeeds

## Contract Verification Tests
- test_precondition_capacity_positive
- test_postcondition_len_reflects_state
- test_invariant_capacity_never_exceeded

## Given-When-Then Scenarios
### Scenario: Enqueue succeeds when queue has capacity
Given: a shard with command_queue_capacity of 4 and command_queue_len of 2
When: enqueue_command is called with a valid command
Then: command_queue_len becomes 3 and Result::Ok(()) is returned

### Scenario: Enqueue fails when queue is full
Given: a shard with command_queue_capacity of 4 and command_queue_len of 4
When: enqueue_command is called
Then: Result::Err(QueueFull) is returned and command_queue_len remains 4
