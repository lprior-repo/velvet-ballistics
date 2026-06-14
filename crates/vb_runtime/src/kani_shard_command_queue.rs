#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for production-bound `ShardCommandQueue` invariants.

use crate::{
    shard::types::{ShardCommand, ShardCommandQueue, MAX_COMMAND_QUEUE_CAPACITY},
    RuntimeError,
};
use vb_core::ids::RunId;

fn arbitrary_queue_command() -> ShardCommand {
    let use_shutdown: bool = kani::any();
    if use_shutdown {
        ShardCommand::Shutdown
    } else {
        ShardCommand::Inspect {
            run: RunId::new(kani::any()),
            correlation: kani::any(),
        }
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_shard_command_queue_bounded_invariant() {
    let capacity: usize = kani::any();
    kani::assume(capacity == 0 || capacity <= 2 || capacity > MAX_COMMAND_QUEUE_CAPACITY);

    let result = ShardCommandQueue::new(capacity);

    if capacity == 0 || capacity > MAX_COMMAND_QUEUE_CAPACITY {
        kani::assert(
            matches!(
                result,
                Err(RuntimeError::CommandQueueCapacityExceeded {
                    capacity: rejected_capacity,
                    max,
                }) if rejected_capacity == capacity && max == MAX_COMMAND_QUEUE_CAPACITY
            ),
            "out-of-domain capacity must be rejected with the bounded error",
        );
        return;
    }

    kani::assert(result.is_ok(), "valid capacity must construct a queue");

    if let Ok(queue) = result {
        kani::assert(
            queue.capacity() == capacity,
            "queue reports constructor capacity",
        );
        kani::assert(queue.len() == 0, "new queue starts empty");
        kani::assert(queue.is_empty(), "new queue is empty");
        kani::assert(!queue.is_full(), "new queue is not full");
        kani::assert(
            queue.remaining_capacity() == capacity,
            "new queue exposes full remaining capacity",
        );
        std::mem::forget(queue);
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_shard_command_queue_push() {
    let capacity: usize = kani::any();
    kani::assume((1..=2).contains(&capacity));

    let queue_result = ShardCommandQueue::new(capacity);
    kani::assert(queue_result.is_ok(), "bounded valid capacity must construct a queue");

    if let Ok(queue) = queue_result {
        let first_result = queue.enqueue(arbitrary_queue_command());
        kani::assert(first_result.is_ok(), "first enqueue must succeed");
        kani::assert(queue.len() == 1, "queue len increments after first enqueue");
        kani::assert(
            queue.remaining_capacity() + queue.len() == capacity,
            "remaining capacity plus len stays equal to capacity",
        );

        if capacity > 1 {
            kani::assert(!queue.is_full(), "queue is not full before reaching capacity");
            let second_result = queue.enqueue(arbitrary_queue_command());
            kani::assert(second_result.is_ok(), "second enqueue must fill the queue");
        }

        kani::assert(queue.len() == capacity, "queue len reaches capacity");
        kani::assert(queue.is_full(), "queue reports full at capacity");
        kani::assert(
            queue.remaining_capacity() == 0,
            "remaining capacity reaches zero at capacity",
        );

        let overflow_result = queue.enqueue(arbitrary_queue_command());
        kani::assert(
            matches!(overflow_result, Err(RuntimeError::QueueFull)),
            "enqueue beyond capacity must return QueueFull",
        );
        kani::assert(
            queue.len() == capacity,
            "overflow enqueue does not change queue length",
        );
        kani::assert(
            queue.remaining_capacity() == 0,
            "overflow enqueue leaves remaining capacity unchanged",
        );
        kani::assert(queue.is_full(), "overflow enqueue leaves queue full");
        std::mem::forget(queue);
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_shard_command_queue_drain() {
    let capacity: usize = kani::any();
    kani::assume((1..=2).contains(&capacity));

    let queue_result = ShardCommandQueue::new(capacity);
    kani::assert(queue_result.is_ok(), "bounded valid capacity must construct a queue");

    if let Ok(queue) = queue_result {
        let first = arbitrary_queue_command();
        let second = arbitrary_queue_command();

        let first_enqueue = queue.enqueue(first.clone());
        kani::assert(first_enqueue.is_ok(), "first enqueue must succeed");

        if capacity > 1 {
            let second_enqueue = queue.enqueue(second.clone());
            kani::assert(second_enqueue.is_ok(), "second enqueue must succeed");
        }

        let first_pop = queue.pop();
        kani::assert(
            first_pop.is_some(),
            "first pop must return the first command",
        );
        if let Some(command) = first_pop {
            kani::assert(command == first, "queue pop preserves FIFO for first item");
        }

        if capacity > 1 {
            let second_pop = queue.pop();
            kani::assert(
                second_pop.is_some(),
                "second pop must return the second command",
            );
            if let Some(command) = second_pop {
                kani::assert(
                    command == second,
                    "queue pop preserves FIFO for second item",
                );
            }
        }

        kani::assert(queue.pop().is_none(), "queue pop returns None once drained");
        kani::assert(queue.is_empty(), "queue is empty after draining all items");
        kani::assert(!queue.is_full(), "drained queue is not full");
        kani::assert(
            queue.remaining_capacity() == capacity,
            "drained queue restores full remaining capacity",
        );
        std::mem::forget(queue);
    }
}
