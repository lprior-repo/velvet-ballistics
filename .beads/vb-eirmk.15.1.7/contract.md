# Contract

The `RunFrame` parallel counter uses checked arithmetic and preserves the complete frame whenever an add or subtract is rejected.

Given a populated three-step, three-slot frame with nondefault program counter, step states, slot values and taints, executed count, maximum 5, and current count 2, lowering the maximum to 1 preserves the current count; adding zero then returns `ParallelLimitExceeded { limit: 1 }` without changing any part of the frame, subtracting one yields current count 1, and adding zero succeeds with current count 1.

Given a frame whose maximum and current count are zero, adding zero succeeds without changing the frame, while adding one returns `ParallelLimitExceeded { limit: 0 }` and preserves the complete frame.

Given a populated frame with maximum and current count at `u16::MAX`, adding one returns `InternalInvariantViolation { reason: "parallel_in_flight overflow" }` and preserves the complete frame.

Given a populated frame with current count 2, subtracting 3 returns `InternalInvariantViolation { reason: "parallel_in_flight underflow" }` and preserves the complete frame.

The behavioral tests use public `RunFrame` methods and compare complete frames on rejected calls; they add coverage for atomicity and counter-limit interactions without changing production behavior or duplicating ordinary increment/decrement cases.
