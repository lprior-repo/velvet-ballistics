//! CLI Postcard Constants
//!
//! vb-k8ut.5: wire-format constants shared across the postcard module.
//! Magic bytes, size bounds, header layout, and schema version.

/// Magic bytes for CLI Postcard format: "VCLA" (Velvet CLI Application).
pub(crate) const CLI_MAGIC: [u8; 4] = [0x56, 0x43, 0x4C, 0x41];

/// Maximum encoded payload size in bytes (64KB).
pub(crate) const MAX_PAYLOAD: usize = 64 * 1024;

pub(crate) const HEADER_SIZE: usize = 52;
pub(crate) const HEADER_SIZE_U32: u32 = 52;
pub(crate) const MAX_PAYLOAD_U32: u32 = 64 * 1024;
pub(crate) const CLI_SCHEMA_VERSION: u16 = 1;
pub(crate) const CLI_POSTCARD_KIND: u16 = 2;
