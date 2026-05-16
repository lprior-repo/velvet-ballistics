#![forbid(unsafe_code)]
//! IPC ingress types.

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};

use vb_core::{RunId, WorkflowDigest};

use crate::{BoundedPayload, MaxPayloadBytes, IpcError, QueueCapacity};

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
    pub(crate) sender: Sender<IngressFrame>,
    pub(crate) receiver: Receiver<IngressFrame>,
}

impl MemoryIngress {
    /// Creates a bounded memory ingress queue.
    #[must_use]
    pub fn bounded(capacity: QueueCapacity) -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(capacity.get());
        Self { sender, receiver }
    }

    /// Attempts to submit a frame without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        self.sender.try_send(frame).map_err(|e| match e {
            TrySendError::Full(_) => IpcError::Full,
            TrySendError::Disconnected(_) => IpcError::Disconnected,
        })
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

    /// Test-only helper that drops the original sender so the channel appears disconnected.
    #[cfg(test)]
    pub(crate) fn disconnect_sender(&mut self) {
        let (new_sender, _) = crossbeam_channel::bounded(1);
        let _ = std::mem::replace(&mut self.sender, new_sender);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use std::num::NonZeroUsize;
    use vb_core::{RunId, WorkflowDigest};

    #[test]
    fn ingress_frame_new_with_empty_payload_and_min_max_succeeds() {
        let min_max = MaxPayloadBytes::new(NonZeroUsize::MIN);
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            Bytes::new(),
            min_max,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn ingress_frame_new_with_payload_exactly_at_max_succeeds() {
        let max = MaxPayloadBytes::DEFAULT;
        let payload = Bytes::from(vec![0u8; max.get()]);
        let result = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            payload,
            max,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn memory_ingress_bounded_capacity_one_accepts_one_rejects_second() {
        let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let frame = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            Bytes::new(),
            MaxPayloadBytes::DEFAULT,
        )
        .unwrap();

        assert!(ingress.try_submit(frame.clone()).is_ok());
        assert!(matches!(ingress.try_submit(frame), Err(IpcError::Full)));
    }

    #[test]
    fn memory_ingress_try_recv_on_empty_queue_returns_none() {
        let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        assert_eq!(ingress.try_recv(), Ok(None));
    }

    #[test]
    fn memory_ingress_try_recv_returns_items_in_fifo_order() {
        let capacity = QueueCapacity::new(NonZeroUsize::new(2).unwrap());
        let ingress = MemoryIngress::bounded(capacity);
        let frame1 = IngressFrame::new(
            RunId::new(1),
            WorkflowDigest::from_bytes([0u8; 32]),
            Bytes::from_static(b"first"),
            MaxPayloadBytes::DEFAULT,
        )
        .unwrap();
        let frame2 = IngressFrame::new(
            RunId::new(2),
            WorkflowDigest::from_bytes([0u8; 32]),
            Bytes::from_static(b"second"),
            MaxPayloadBytes::DEFAULT,
        )
        .unwrap();

        ingress.try_submit(frame1.clone()).unwrap();
        ingress.try_submit(frame2.clone()).unwrap();

        assert_eq!(ingress.try_recv(), Ok(Some(frame1)));
        assert_eq!(ingress.try_recv(), Ok(Some(frame2)));
        assert_eq!(ingress.try_recv(), Ok(None));
    }

    #[test]
    fn memory_ingress_recv_returns_disconnected_after_sender_drop() {
        let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
        let mut ingress = MemoryIngress::bounded(capacity);
        ingress.disconnect_sender();
        assert!(matches!(ingress.try_recv(), Err(IpcError::Disconnected)));
    }
}
