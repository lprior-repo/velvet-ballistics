#![forbid(unsafe_code)]
//! IPC ingress types.

use bytes::Bytes;
use crossbeam_channel::{Receiver, Sender, TryRecvError, TrySendError};

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

/// Cloneable producer handle for a bounded memory ingress queue.
#[derive(Debug, Clone)]
pub struct MemoryIngressSender {
    sender: Sender<IngressFrame>,
}

impl MemoryIngressSender {
    /// Attempts to submit a frame through this producer handle without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        submit_to_sender(&self.sender, frame)
    }
}

/// Bounded multi-producer, single-consumer memory ingress queue.
#[derive(Debug)]
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

    /// Creates an additional producer handle sharing this queue's bounded buffer.
    #[must_use]
    pub fn producer(&self) -> MemoryIngressSender {
        MemoryIngressSender {
            sender: self.sender.clone(),
        }
    }

    /// Attempts to submit a frame without blocking.
    pub fn try_submit(&self, frame: IngressFrame) -> Result<(), IpcError> {
        submit_to_sender(&self.sender, frame)
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

    #[cfg(test)]
    pub(crate) fn disconnect_sender(&mut self) {
        let (new_sender, _) = crossbeam_channel::bounded(1);
        self.sender = new_sender;
    }
}

fn submit_to_sender(sender: &Sender<IngressFrame>, frame: IngressFrame) -> Result<(), IpcError> {
    sender.try_send(frame).map_err(|e| match e {
        TrySendError::Full(_) => IpcError::Full,
        TrySendError::Disconnected(_) => IpcError::Disconnected,
    })
}

#[cfg(test)]
#[path = "ingress/tests.rs"]
mod tests;
