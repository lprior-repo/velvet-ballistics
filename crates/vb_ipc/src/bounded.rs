#![forbid(unsafe_code)]
//! Bounded payload types.

use bytes::Bytes;
use std::num::NonZeroUsize;

use crate::error::IpcError;

/// Queue capacity for memory ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct QueueCapacity(NonZeroUsize);

impl QueueCapacity {
    /// Creates a non-zero queue capacity.
    pub const fn new(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub(crate) fn get(self) -> usize {
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

    pub(crate) fn get(self) -> usize {
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

/// Bounds for a read operation extent in an IPC buffer.
///
/// Defines the start offset and length of a bounded read region
/// within an IPC buffer frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedReadExtent(
    /// Starting offset within the buffer.
    usize,
    /// Length of the read extent.
    usize,
);

impl BoundedReadExtent {
    /// Creates a new read extent with offset and length.
    #[must_use]
    pub const fn new(offset: usize, length: usize) -> Option<Self> {
        Some(Self(offset, length))
    }

    /// Returns the start offset.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.0
    }

    /// Returns the length.
    #[must_use]
    pub const fn length(self) -> usize {
        self.1
    }

    /// Returns the end offset (offset + length).
    #[must_use]
    pub const fn end(self) -> usize {
        self.0.saturating_add(self.1)
    }
}

/// Bounds for a write/drain operation extent in an IPC buffer.
///
/// Defines the start offset and capacity bound for a bounded write
/// or drain region within an IPC buffer frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedWriteDrainExtent(
    /// Starting offset within the buffer.
    usize,
    /// Capacity bound for the write/drain operation.
    usize,
);

impl BoundedWriteDrainExtent {
    /// Creates a new write/drain extent with offset and capacity.
    #[must_use]
    pub const fn new(offset: usize, capacity: usize) -> Option<Self> {
        Some(Self(offset, capacity))
    }

    /// Returns the start offset.
    #[must_use]
    pub const fn offset(self) -> usize {
        self.0
    }

    /// Returns the capacity bound.
    #[must_use]
    pub const fn capacity(self) -> usize {
        self.1
    }

    /// Returns the end offset (offset + capacity).
    #[must_use]
    pub const fn end(self) -> usize {
        self.0.saturating_add(self.1)
    }
}
