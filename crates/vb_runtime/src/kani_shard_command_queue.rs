#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for the shard-command queue constructor and private stand-in model.
//!
//! These harnesses prove the shared constructor predicate plus bounded
//! FIFO/fullness/error behavior of a private sequential model defined in this
//! proof module. The model exists only to keep Kani tractable; it shares the
//! public capacity domain but does not execute production
//! `crossbeam_queue::ArrayQueue` logic and must not be treated as production
//! queue proof.

use std::cell::Cell;

use crate::{
    RuntimeError,
    shard::types::{MAX_COMMAND_QUEUE_CAPACITY, is_valid_command_queue_capacity},
};

const KANI_MODEL_SLOT_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KaniShardCommandToken {
    Inspect { run: u64, correlation: u64 },
    Shutdown,
}

struct KaniShardCommandQueueModel {
    slots: Cell<[Option<KaniShardCommandToken>; KANI_MODEL_SLOT_COUNT]>,
    head: Cell<usize>,
    len: Cell<usize>,
    capacity: usize,
}

impl KaniShardCommandQueueModel {
    fn new(capacity: usize) -> Result<Self, RuntimeError> {
        if !is_valid_command_queue_capacity(capacity) {
            return Err(RuntimeError::CommandQueueCapacityExceeded {
                capacity,
                max: MAX_COMMAND_QUEUE_CAPACITY,
            });
        }

        Ok(Self {
            slots: Cell::new([None; KANI_MODEL_SLOT_COUNT]),
            head: Cell::new(0),
            len: Cell::new(0),
            capacity,
        })
    }

    fn enqueue_token(&self, token: KaniShardCommandToken) -> Result<(), RuntimeError> {
        let len = self.len.get();
        if len >= self.capacity {
            return Err(RuntimeError::QueueFull);
        }

        let Some(slot_index) = self.slot_index_for_offset(len) else {
            return Err(RuntimeError::QueueFull);
        };
        let Some(next_len) = len.checked_add(1) else {
            return Err(RuntimeError::QueueFull);
        };

        let mut slots = self.slots.get();
        let Some(slot) = slots.get_mut(slot_index) else {
            return Err(RuntimeError::QueueFull);
        };
        if slot.is_some() {
            return Err(RuntimeError::QueueFull);
        }

        *slot = Some(token);
        self.slots.set(slots);
        self.len.set(next_len);
        Ok(())
    }

    fn pop_token(&self) -> Option<KaniShardCommandToken> {
        let len = self.len.get();
        if len == 0 {
            return None;
        }

        let head = self.head.get();
        let command = {
            let mut slots = self.slots.get();
            let slot = slots.get_mut(head)?;
            let token = slot.take();
            self.slots.set(slots);
            token
        };

        if command.is_some() {
            let next_len = len.saturating_sub(1);
            self.len.set(next_len);
            if next_len == 0 {
                self.head.set(0);
            } else {
                let Some(next_head) = self.next_model_head(head) else {
                    return command;
                };
                self.head.set(next_head);
            }
        }

        command
    }

    fn len(&self) -> usize {
        self.len.get()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    fn is_full(&self) -> bool {
        self.len() == self.capacity
    }

    fn slot_index_for_offset(&self, offset: usize) -> Option<usize> {
        if offset >= self.capacity {
            return None;
        }

        let span = self.model_slot_span();
        if span == 0 {
            return None;
        }

        let sum = self.head.get().checked_add(offset)?;
        let index = if sum < span {
            Some(sum)
        } else {
            sum.checked_sub(span)
        }?;
        if index < span { Some(index) } else { None }
    }

    fn next_model_head(&self, head: usize) -> Option<usize> {
        let span = self.model_slot_span();
        if span == 0 {
            return None;
        }

        let next = head.checked_add(1)?;
        if next < span { Some(next) } else { Some(0) }
    }

    const fn model_slot_span(&self) -> usize {
        if self.capacity < KANI_MODEL_SLOT_COUNT {
            self.capacity
        } else {
            KANI_MODEL_SLOT_COUNT
        }
    }
}

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
    kani::assume(capacity == 0 || capacity <= 3 || capacity > MAX_COMMAND_QUEUE_CAPACITY);
    kani::cover!(capacity == 0, "zero capacity rejection covered");
    kani::cover!(capacity == 1, "minimum valid capacity covered");
    kani::cover!(capacity == 2, "two-slot constructor case covered");
    kani::cover!(capacity == 3, "three-slot constructor case covered");
    kani::cover!(
        capacity > MAX_COMMAND_QUEUE_CAPACITY,
        "over-maximum capacity rejection covered"
    );

    if capacity == 0 || capacity > MAX_COMMAND_QUEUE_CAPACITY {
        match KaniShardCommandQueueModel::new(capacity) {
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

    match KaniShardCommandQueueModel::new(capacity) {
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

    match KaniShardCommandQueueModel::new(capacity) {
        Ok(queue) => {
            let first_result = queue.enqueue_token(arbitrary_queue_token());
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
                match queue.enqueue_token(arbitrary_queue_token()) {
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

            match queue.enqueue_token(arbitrary_queue_token()) {
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

    match KaniShardCommandQueueModel::new(capacity) {
        Ok(queue) => {
            let first = arbitrary_queue_token();
            let second = arbitrary_queue_token();

            let first_enqueue = queue.enqueue_token(first);
            match first_enqueue {
                Ok(()) => {}
                Err(error) => {
                    std::mem::forget(error);
                    kani::assert(false, "first enqueue must succeed");
                }
            }

            if capacity > 1 {
                let second_enqueue = queue.enqueue_token(second);
                match second_enqueue {
                    Ok(()) => {}
                    Err(error) => {
                        std::mem::forget(error);
                        kani::assert(false, "second enqueue must succeed");
                    }
                }
            }

            let first_pop = queue.pop_token();
            kani::assert(
                first_pop.is_some(),
                "first pop must return the first command",
            );
            if let Some(command) = first_pop {
                kani::assert(command == first, "queue pop preserves FIFO for first item");
            }

            if capacity > 1 {
                let second_pop = queue.pop_token();
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
                queue.pop_token().is_none(),
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
