#![forbid(unsafe_code)]
//! Allocation-free validation of the historical tag-46 payload.

use crate::{JournalError, RecordKind, constants::DIGEST_BYTES};
use serde::Deserialize;
use vb_core::capability_grant::{
    MAX_GRANT_CAPABILITIES, MAX_GRANT_RESOURCES, MAX_GRANT_SIGNATURE_LEN, MAX_GRANT_TEXT_LEN,
};

pub(super) fn validate(payload: &[u8]) -> Result<(), JournalError> {
    let mut cursor = Cursor::new(payload);
    if cursor.scalar::<u16>()? != RecordKind::CapabilityGrant.id() {
        return Err(JournalError::InvalidEvent);
    }
    let run = cursor.scalar::<u64>()?;
    let seq = cursor.scalar::<u64>()?;
    if run == 0 || seq == u64::MAX {
        return Err(JournalError::InvalidEvent);
    }

    // The following reads preserve the historical fifteen-field DTO order.
    cursor.scalar::<u64>()?; // GrantId
    cursor.text(MAX_GRANT_TEXT_LEN)?; // issuer
    cursor.text(MAX_GRANT_TEXT_LEN)?; // subject
    cursor.bytes(DIGEST_BYTES)?; // artifact digest
    if cursor.scalar::<u64>()? != run {
        return Err(JournalError::InvalidEvent);
    }
    cursor.scalar::<u16>()?; // ActionId

    let capabilities = cursor.length(MAX_GRANT_CAPABILITIES, true)?;
    for _ in 0..capabilities {
        // Historical names have no per-name text cap. Borrowed input and the
        // shared copied-byte ledger bound them by this actual payload instead.
        cursor.text(payload.len())?;
        cursor.scalar::<u16>()?;
    }
    let resources = cursor.length(MAX_GRANT_RESOURCES, false)?;
    for _ in 0..resources {
        cursor.text(MAX_GRANT_TEXT_LEN)?;
    }
    let issued_at_ms = cursor.scalar::<u64>()?;
    let expires_at_ms = cursor.scalar::<u64>()?;
    if issued_at_ms >= expires_at_ms {
        return Err(JournalError::InvalidEvent);
    }
    cursor.scalar::<bool>()?; // delegable
    cursor.scalar::<u64>()?; // revocation epoch
    cursor.scalar::<bool>()?; // break_glass
    if cursor.option()? {
        cursor.bytes(DIGEST_BYTES)?; // approval receipt digest
    }
    if cursor.option()? {
        let signature_len = cursor.length(MAX_GRANT_SIGNATURE_LEN, true)?;
        cursor.bytes(signature_len)?;
    }
    if !cursor.remaining.is_empty() {
        return Err(JournalError::InvalidEvent);
    }
    Ok(())
}

/// This ledger bounds bytes copied from variable fields and fixed digests by
/// the accepted payload length. It is not a bound on allocator overhead or RSS.
struct Cursor<'payload> {
    remaining: &'payload [u8],
    copy_bytes_left: usize,
}

impl<'payload> Cursor<'payload> {
    const fn new(payload: &'payload [u8]) -> Self {
        Self {
            remaining: payload,
            copy_bytes_left: payload.len(),
        }
    }

    // Callers use only scalar integer, bool and Option<()> types here.
    fn scalar<T: Deserialize<'payload>>(&mut self) -> Result<T, JournalError> {
        let (value, remaining) = postcard::take_from_bytes(self.remaining)
            .map_err(JournalError::PostcardDecodeFailed)?;
        self.remaining = remaining;
        Ok(value)
    }

    fn length(&mut self, max: usize, require_nonempty: bool) -> Result<usize, JournalError> {
        let length =
            usize::try_from(self.scalar::<u64>()?).map_err(|_| JournalError::InvalidEvent)?;
        if length > max || (require_nonempty && length == 0) {
            return Err(JournalError::InvalidEvent);
        }
        Ok(length)
    }

    fn bytes(&mut self, length: usize) -> Result<&'payload [u8], JournalError> {
        let (value, remaining) = self
            .remaining
            .split_at_checked(length)
            .ok_or(JournalError::InvalidEvent)?;
        self.copy_bytes_left = self
            .copy_bytes_left
            .checked_sub(length)
            .ok_or(JournalError::InvalidEvent)?;
        self.remaining = remaining;
        Ok(value)
    }

    fn text(&mut self, max: usize) -> Result<(), JournalError> {
        let length = self.length(max, true)?;
        std::str::from_utf8(self.bytes(length)?)
            .map(|_| ())
            .map_err(|_| JournalError::InvalidEvent)
    }

    fn option(&mut self) -> Result<bool, JournalError> {
        self.scalar::<Option<()>>().map(|value| value.is_some())
    }
}
