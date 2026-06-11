#![forbid(unsafe_code)]
//! IPC ingress types.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use bytes::Bytes;
use crossbeam_queue::ArrayQueue;

use vb_core::{RunId, WorkflowDigest};

use crate::{BoundedPayload, IpcError, MaxPayloadBytes, QueueCapacity};

/// Binary frame submitted by an in-process or IPC producer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressFrame {
    run_id: RunId,
    workflow: WorkflowDigest,
    payload: BoundedPayload,
}

impl IngressFrame {
    /// Creates a frame after applying the payload size contract.
    pub fn new(
        run_id: RunId,
        workflow: WorkflowDigest,
        payload: Bytes,
        max_payload: MaxPayloadBytes,
    ) -> Result<Self, IpcError> {
        Ok(Self {
            run_id,
            workflow,
            payload: BoundedPayload::new(payload, max_payload)?,
        })
    }

    /// Run identifier selected by the caller or allocator.
    #[must_use]
    pub const fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Compiled workflow digest this frame targets.
    #[must_use]
    pub const fn workflow(&self) -> WorkflowDigest {
        self.workflow
    }

    /// Raw input bytes. Parsing/mapping is a cold boundary concern.
    #[must_use]
    pub const fn payload(&self) -> &BoundedPayload {
        &self.payload
    }
}

/// Lock-free bounded MPSC queue core: `ArrayQueue<IngressFrame>` for hot-path
/// throughput (master §50; no `Mutex` on the hot path per §4), an atomic
/// disconnect flag set when the last consumer or the last producer drops,
/// and per-side refcounts that drive the disconnect signal.
#[derive(Debug)]
pub(crate) struct IngressCore {
    queue: ArrayQueue<IngressFrame>,
    disconnected: AtomicBool,
    consumer_count: AtomicUsize,
    producer_count: AtomicUsize,
}

impl IngressCore {
    fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            disconnected: AtomicBool::new(false),
            consumer_count: AtomicUsize::new(0),
            producer_count: AtomicUsize::new(0),
        }
    }

    fn try_push(&self, frame: IngressFrame) -> Result<(), IpcError> {
        // Check disconnect first so a full-and-disconnected queue returns
        // Disconnected, not Full — preserves the crossbeam_channel error
        // priority.
        if self.disconnected.load(Ordering::Acquire) {
            return Err(IpcError::Disconnected);
        }
        match self.queue.push(frame) {
            Ok(()) => Ok(()),
            Err(_) => {
                if self.disconnected.load(Ordering::Acquire) {
                    Err(IpcError::Disconnected)
                } else {
                    Err(IpcError::Full)
                }
            }
        }
    }

    fn try_pop(&self) -> Result<Option<IngressFrame>, IpcError> {
        match self.queue.pop() {
            Some(frame) => Ok(Some(frame)),
            None => {
                if self.disconnected.load(Ordering::Acquire) {
                    Err(IpcError::Disconnected)
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn disconnect(&self) {
        self.disconnected.store(true, Ordering::Release);
    }
}

/// Cloneable producer handle for a bounded memory ingress queue.
#[derive(Debug, Clone)]
pub struct MemoryIngressSender {
    core: Arc<IngressCore>,
}

impl MemoryIngressSender {
    /// Attempts to submit a frame through this producer handle without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        self.core.try_push(frame)
    }
}

impl Drop for MemoryIngressSender {
    fn drop(&mut self) {
        // Decrement the producer refcount. When the last producer drops
        // (count transitions 1 -> 0), flip the disconnect flag so any
        // remaining consumer can observe `Disconnected` from `try_recv`.
        // `fetch_sub` returns the previous value, so `prev == 1` means we
        // are the last producer; this is race-free across concurrent drops.
        let prev = self.core.producer_count.fetch_sub(1, Ordering::AcqRel);
        if prev <= 1 {
            self.core.disconnect();
        }
    }
}

/// Bounded multi-producer, single-consumer memory ingress queue.
#[derive(Debug, Clone)]
pub struct MemoryIngress {
    core: Arc<IngressCore>,
}

impl MemoryIngress {
    /// Creates a bounded memory ingress queue.
    #[must_use]
    pub fn bounded(capacity: QueueCapacity) -> Self {
        Self {
            core: Arc::new(IngressCore::new(capacity.get())),
        }
    }

    /// Creates an additional producer handle sharing this queue's bounded buffer.
    #[must_use]
    pub fn producer(&self) -> MemoryIngressSender {
        self.core.producer_count.fetch_add(1, Ordering::AcqRel);
        MemoryIngressSender {
            core: Arc::clone(&self.core),
        }
    }

    /// Attempts to submit a frame without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        self.core.try_push(frame)
    }

    /// Attempts to receive one frame without blocking.
    pub fn try_recv(&self) -> Result<Option<IngressFrame>, IpcError> {
        self.core.try_pop()
    }

    /// Current approximate queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.core.len()
    }

    /// Returns true when no frames are queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.core.is_empty()
    }

    /// Internal accessor for tests and shared consumers that need the core.
    #[cfg(test)]
    pub(crate) fn core(&self) -> Arc<IngressCore> {
        Arc::clone(&self.core)
    }

    /// Disconnect flag for tests; flips the atomic so producers see
    /// `Disconnected` on the next push.
    #[cfg(test)]
    pub(crate) fn disconnect_sender(&mut self) {
        self.core.disconnect();
    }
}

impl Drop for MemoryIngress {
    fn drop(&mut self) {
        // Increment-then-decrement pattern: when the last consumer drops,
        // flip the disconnect flag. We do this with a simple fetch_add /
        // fetch_sub + post-decrement check; if no consumers are ever
        // created (producer-only path), the count is 0 on entry and
        // we still flip the flag on drop, which is the correct
        // "all-receivers-dropped" signal.
        let prev = self.core.consumer_count.fetch_add(1, Ordering::AcqRel);
        // The new "active" consumer count includes this one. We then
        // immediately decrement on drop. If we transition from 1 to 0,
        // we are the last consumer and must signal disconnect.
        // Using a second atomic op (fetch_sub) is simpler than carrying
        // a per-instance counter and matches the existing MPSC receiver
        // pattern in vb_runtime::action_queue.
        let after = self.core.consumer_count.fetch_sub(1, Ordering::AcqRel);
        if after == 1 && prev == 0 {
            // We were the only consumer; flip the disconnect flag.
            self.core.disconnect();
        } else if after == 0 {
            // Transition from 1 active to 0 after our add+sub: last
            // consumer. Flip.
            self.core.disconnect();
        }
    }
}

#[cfg(test)]
#[path = "ingress/tests.rs"]
mod tests;
