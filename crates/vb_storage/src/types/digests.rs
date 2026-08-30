#![forbid(unsafe_code)]
//! Digest newtypes and status-byte types for vb_storage.
//!
//! All 32-byte BLAKE3 digests used in the storage layer are represented as
//! distinct newtypes so that the compiler prevents accidental mixing of
//! workflow-source, compiled-IR, blob, payload, and value digests.
//!
//! Status-byte types prevent the named `IndexStatusState` variants from
//! colliding with raw `u8` values that would round-trip to the wrong enum
//! member on decode (SC-001 / vb-f1xkn).

use crate::constants::DIGEST_BYTES;

// ============================================================================
// Payload digests (record checksums)
// ============================================================================

/// BLAKE3 digest of a journal-record payload stored in a [`RecordHeader`].
///
/// The digest is 32 bytes (256 bits) wide and is computed over the raw
/// payload bytes at record-encode time so that every decode can verify
/// payload integrity without a second hash pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct PayloadDigest([u8; DIGEST_BYTES]);

impl PayloadDigest {
    /// Creates a payload digest from already-computed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_BYTES] {
        &self.0
    }

    /// Returns the raw digest bytes as an array.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl From<[u8; DIGEST_BYTES]> for PayloadDigest {
    fn from(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self::from_bytes(bytes)
    }
}

// ============================================================================
// Artifact digests (storage keys)
// ============================================================================

/// BLAKE3 digest identifying a workflow-source document in the `workflow_source` keyspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct WorkflowSourceDigest([u8; DIGEST_BYTES]);

impl WorkflowSourceDigest {
    /// Creates a workflow-source digest from already-computed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_BYTES] {
        &self.0
    }

    /// Returns the raw digest bytes as an array.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl From<[u8; DIGEST_BYTES]> for WorkflowSourceDigest {
    fn from(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self::from_bytes(bytes)
    }
}

/// BLAKE3 digest identifying a compiled-IR artifact in the `compiled_ir` keyspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct CompiledIrDigest([u8; DIGEST_BYTES]);

impl CompiledIrDigest {
    /// Creates a compiled-IR digest from already-computed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_BYTES] {
        &self.0
    }

    /// Returns the raw digest bytes as an array.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl From<[u8; DIGEST_BYTES]> for CompiledIrDigest {
    fn from(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self::from_bytes(bytes)
    }
}

/// BLAKE3 digest identifying a blob in the `blob` keyspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct BlobDigest([u8; DIGEST_BYTES]);

impl BlobDigest {
    /// Creates a blob digest from already-computed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_BYTES] {
        &self.0
    }

    /// Returns the raw digest bytes as an array.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl From<[u8; DIGEST_BYTES]> for BlobDigest {
    fn from(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self::from_bytes(bytes)
    }
}

// ============================================================================
// Value digests (action-envelope verification)
// ============================================================================

/// BLAKE3 digest of an action-envelope output value, used for replay divergence detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct ValueDigest([u8; DIGEST_BYTES]);

impl ValueDigest {
    /// Creates a value digest from already-computed bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_BYTES] {
        &self.0
    }

    /// Returns the raw digest bytes as an array.
    #[must_use]
    pub const fn to_bytes(self) -> [u8; DIGEST_BYTES] {
        self.0
    }
}

impl From<[u8; DIGEST_BYTES]> for ValueDigest {
    fn from(bytes: [u8; DIGEST_BYTES]) -> Self {
        Self::from_bytes(bytes)
    }
}

// ============================================================================
// Status-byte newtypes (non-colliding index-state markers)
// ============================================================================

/// Non-colliding status byte for `index_status` key encoding.
///
/// This newtype wraps a single `u8` that is used as a state marker in
/// `index_status` storage keys. The named variants of `IndexStatusState`
/// occupy bytes `0..MIN_OTHER_STATUS_BYTE`; any raw value below that
/// threshold is rejected by the encoder to prevent collision-based
/// misdecoding (SC-001 / vb-f1xkn).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StatusByte(u8);

impl StatusByte {
    /// Creates a status byte, rejecting values in the collision range.
    ///
    /// Returns `Err` if `value` is below `MIN_OTHER_STATUS_BYTE` (i.e. 0, 1, 2),
    /// which collide with `Submitted`, `Active`, and `Completed`.
    pub fn try_new(value: u8) -> Result<Self, crate::JournalError> {
        if value < crate::constants::MIN_OTHER_STATUS_BYTE {
            return Err(crate::JournalError::IndexStatusStateCollision {
                byte: value,
                min: crate::constants::MIN_OTHER_STATUS_BYTE,
            });
        }
        Ok(Self(value))
    }

    /// Creates a status byte from a raw value without collision checking.
    ///
    /// Only use this when you already know the byte is safe (e.g., decoding
    /// a value that was previously encoded via `to_u8`).
    #[must_use]
    pub const fn unchecked(value: u8) -> Self {
        Self(value)
    }

    /// Returns the raw byte value.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl From<StatusByte> for u8 {
    fn from(val: StatusByte) -> Self {
        val.get()
    }
}

impl From<u8> for StatusByte {
    fn from(value: u8) -> Self {
        Self::unchecked(value)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for StatusByte {
    fn any() -> Self {
        let value: u8 = kani::any();
        Self::unchecked(value)
    }
}
