//! Bounded memory ingress for Velvet Ballistics.
//!
//! This crate deliberately exposes memory/IPC-shaped primitives only. HTTP is
//! not part of the hot control plane.

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError, bounded};
use std::num::NonZeroUsize;
use thiserror::Error;
use vb_core::{RunId, WorkflowDigest};

/// Queue capacity for memory ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct QueueCapacity(NonZeroUsize);

impl QueueCapacity {
    /// Creates a non-zero queue capacity.
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    fn get(self) -> usize {
        self.0.get()
    }
}

/// Maximum accepted payload bytes for an ingress frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaxPayloadBytes(NonZeroUsize);

impl MaxPayloadBytes {
    /// Default single-frame payload bound: 1 MiB.
    pub const DEFAULT: Self = Self(match NonZeroUsize::new(1_048_576) {
        Some(value) => value,
        None => NonZeroUsize::MIN,
    });

    /// Creates a non-zero payload limit.
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    fn get(self) -> usize {
        self.0.get()
    }
}

/// Payload accepted after a caller-visible size check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedPayload(Bytes);

impl BoundedPayload {
    /// Creates a checked bounded payload.
    pub fn new(payload: Bytes, max: MaxPayloadBytes) -> Result<Self, IpcError> {
        if payload.len() > max.get() {
            Err(IpcError::PayloadTooLarge {
                actual: payload.len(),
                limit: max.get(),
            })
        } else {
            Ok(Self(payload))
        }
    }

    /// Returns shared payload bytes.
    #[must_use]
    pub const fn bytes(&self) -> &Bytes {
        &self.0
    }
}

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

/// Bounded multi-producer, single-consumer memory ingress queue.
#[derive(Debug, Clone)]
pub struct MemoryIngress {
    sender: Sender<IngressFrame>,
    receiver: Receiver<IngressFrame>,
}

impl MemoryIngress {
    /// Creates a bounded memory ingress queue.
    #[must_use]
    pub fn bounded(capacity: QueueCapacity) -> Self {
        let (sender, receiver) = bounded(capacity.get());
        Self { sender, receiver }
    }

    /// Attempts to submit a frame without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        self.sender.try_send(frame).map_err(map_try_send)
    }

    /// Attempts to receive one frame without blocking.
    pub fn try_recv(&self) -> Result<Option<IngressFrame>, IpcError> {
        match self.receiver.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(IpcError::Disconnected),
        }
    }

    /// Current approximate queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Returns true when no frames are queued.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
}

/// IPC/memory ingress failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IpcError {
    /// Queue is full and the producer must apply backpressure.
    #[error("memory ingress queue is full")]
    Full,
    /// All producers or consumers have disconnected.
    #[error("memory ingress queue is disconnected")]
    Disconnected,
    /// Payload exceeds the configured frame limit.
    #[error("ingress payload is too large: actual={actual}, limit={limit}")]
    PayloadTooLarge {
        /// Actual payload bytes.
        actual: usize,
        /// Configured limit.
        limit: usize,
    },
}

fn map_try_send(error: TrySendError<IngressFrame>) -> IpcError {
    match error {
        TrySendError::Full(_) => IpcError::Full,
        TrySendError::Disconnected(_) => IpcError::Disconnected,
    }
}

#[cfg(test)]
mod tests {
    use super::{IngressFrame, IpcError, MaxPayloadBytes, MemoryIngress, QueueCapacity};
    use bytes::Bytes;
    use vb_core::{RunId, WorkflowDigest};

    #[test]
    fn bounded_queue_applies_backpressure() {
        let capacity = QueueCapacity::new(std::num::NonZeroUsize::MIN);
        let queue = MemoryIngress::bounded(capacity);
        let frame = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([2; 32]),
            Bytes::from_static(b"{}"),
            MaxPayloadBytes::DEFAULT,
        );
        assert!(frame.is_ok(), "test frame should fit default payload bound");
        let Ok(frame) = frame else {
            return;
        };

        assert_eq!(queue.try_submit(frame.clone()), Ok(()));
        assert_eq!(queue.try_submit(frame), Err(IpcError::Full));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn oversized_payload_is_rejected() {
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([2; 32]),
            Bytes::from_static(b"too big"),
            MaxPayloadBytes::new(std::num::NonZeroUsize::MIN),
        );

        assert!(matches!(result, Err(IpcError::PayloadTooLarge { .. })));
    }
}
