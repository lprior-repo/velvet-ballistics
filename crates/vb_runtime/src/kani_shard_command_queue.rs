#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for production-bound `ShardCommandQueue` invariants.

use crate::{
    RuntimeError,
    shard::{
        queue::KaniShardCommandToken,
        types::{MAX_COMMAND_QUEUE_CAPACITY, ShardCommandQueue},
    },
};

fn arbitrary_queue_token() -> KaniShardCommandToken {
    let use_shutdown: bool = kani::any();
    if use_shutdown {
        KaniShardCommandToken::Shutdown
    } else {
        KaniShardCommandToken::Inspect {
            run: kani::any(),
            correlation: kani::any(),
        }
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_shard_command_queue_bounded_invariant() {
    let capacity: usize = kani::any();
    kani::assume(capacity == 0 || capacity <= 2 || capacity > MAX_COMMAND_QUEUE_CAPACITY);
    kani::cover!(capacity == 0, "zero capacity rejection covered");
    kani::cover!(capacity == 1, "minimum valid capacity covered");
    kani::cover!(capacity == 2, "maximum Kani model capacity covered");
    kani::cover!(
        capacity > MAX_COMMAND_QUEUE_CAPACITY,
        "over-maximum capacity rejection covered"
    );

    if capacity == 0 || capacity > MAX_COMMAND_QUEUE_CAPACITY {
        match ShardCommandQueue::new(capacity) {
            Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity: rejected_capacity,
                max,
            }) => {
                kani::assert(
                    rejected_capacity == capacity && max == MAX_COMMAND_QUEUE_CAPACITY,
                    "out-of-domain capacity must be rejected with the bounded error",
                );
            }
            Err(error) => {
                std::mem::forget(error);
                kani::assert(false, "out-of-domain capacity returned the wrong error");
            }
            Ok(queue) => {
                std::mem::forget(queue);
                kani::assert(false, "out-of-domain capacity constructed a queue");
            }
        }
        return;
    }

    match ShardCommandQueue::new(capacity) {
        Ok(queue) => {
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
        Err(error) => {
            std::mem::forget(error);
            kani::assert(false, "valid capacity must construct a queue");
        }
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_shard_command_queue_push() {
    let capacity: usize = kani::any();
    kani::assume((1..=2).contains(&capacity));
    kani::cover!(capacity == 1, "single-slot push domain covered");
    kani::cover!(capacity == 2, "two-slot push domain covered");

    match ShardCommandQueue::new(capacity) {
        Ok(queue) => {
            let first_result = queue.enqueue_kani_token(arbitrary_queue_token());
            match first_result {
                Ok(()) => {}
                Err(error) => {
                    std::mem::forget(error);
                    kani::assert(false, "first enqueue must succeed");
                }
            }
            kani::assert(queue.len() == 1, "queue len increments after first enqueue");
            kani::assert(
                queue.remaining_capacity() + queue.len() == capacity,
                "remaining capacity plus len stays equal to capacity",
            );

            if capacity > 1 {
                kani::assert(
                    !queue.is_full(),
                    "queue is not full before reaching capacity",
                );
                match queue.enqueue_kani_token(arbitrary_queue_token()) {
                    Ok(()) => {}
                    Err(error) => {
                        std::mem::forget(error);
                        kani::assert(false, "second enqueue must fill the queue");
                    }
                }
            }

            kani::assert(queue.len() == capacity, "queue len reaches capacity");
            kani::assert(queue.is_full(), "queue reports full at capacity");
            kani::assert(
                queue.remaining_capacity() == 0,
                "remaining capacity reaches zero at capacity",
            );

            match queue.enqueue_kani_token(arbitrary_queue_token()) {
                Err(RuntimeError::QueueFull) => {}
                Err(error) => {
                    std::mem::forget(error);
                    kani::assert(false, "enqueue beyond capacity returned wrong error");
                }
                Ok(()) => kani::assert(false, "enqueue beyond capacity must return QueueFull"),
            }
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
        Err(error) => {
            std::mem::forget(error);
            kani::assert(false, "bounded valid capacity must construct a queue");
        }
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_shard_command_queue_drain() {
    let capacity: usize = kani::any();
    kani::assume((1..=2).contains(&capacity));
    kani::cover!(capacity == 1, "single-slot drain domain covered");
    kani::cover!(capacity == 2, "two-slot drain domain covered");

    match ShardCommandQueue::new(capacity) {
        Ok(queue) => {
            let first = arbitrary_queue_token();
            let second = arbitrary_queue_token();

            let first_enqueue = queue.enqueue_kani_token(first);
            match first_enqueue {
                Ok(()) => {}
                Err(error) => {
                    std::mem::forget(error);
                    kani::assert(false, "first enqueue must succeed");
                }
            }

            if capacity > 1 {
                let second_enqueue = queue.enqueue_kani_token(second);
                match second_enqueue {
                    Ok(()) => {}
                    Err(error) => {
                        std::mem::forget(error);
                        kani::assert(false, "second enqueue must succeed");
                    }
                }
            }

            let first_pop = queue.pop_kani_token();
            kani::assert(
                first_pop.is_some(),
                "first pop must return the first command",
            );
            if let Some(command) = first_pop {
                kani::assert(command == first, "queue pop preserves FIFO for first item");
            }

            if capacity > 1 {
                let second_pop = queue.pop_kani_token();
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

            kani::assert(
                queue.pop_kani_token().is_none(),
                "queue pop returns None once drained",
            );
            kani::assert(queue.is_empty(), "queue is empty after draining all items");
            kani::assert(!queue.is_full(), "drained queue is not full");
            kani::assert(
                queue.remaining_capacity() == capacity,
                "drained queue restores full remaining capacity",
            );
            std::mem::forget(queue);
        }
        Err(error) => {
            std::mem::forget(error);
            kani::assert(false, "bounded valid capacity must construct a queue");
        }
    }
}
