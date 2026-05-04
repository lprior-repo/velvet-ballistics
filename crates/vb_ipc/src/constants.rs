//! IPC protocol constants.

/// IPC frame magic: `VBLT` little-endian.
pub const IPC_MAGIC: u32 = 0x5642_4C54;
/// Supported IPC schema version.
pub const IPC_VERSION: u16 = 1;
/// Fixed IPC header length in bytes.
pub const IPC_HEADER_LEN: usize = 24;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_magic_is_vblt_little_endian() {
        let magic_bytes = IPC_MAGIC.to_le_bytes();
        assert_eq!(magic_bytes, [0x54, 0x4C, 0x42, 0x56]);
    }

    #[test]
    fn ipc_magic_is_non_zero() {
        assert_ne!(IPC_MAGIC, 0);
    }

    #[test]
    fn ipc_version_is_one() {
        assert_eq!(IPC_VERSION, 1);
    }

    #[test]
    fn ipc_header_len_is_24() {
        assert_eq!(IPC_HEADER_LEN, 24);
    }

    #[test]
    fn ipc_header_len_matches_wire_layout() {
        // Wire layout: magic(4) + version(2) + command(2) + flags(2) + reserved(2) + correlation(8) + payload_len(4) = 24
        let expected = 4 + 2 + 2 + 2 + 2 + 8 + 4;
        assert_eq!(IPC_HEADER_LEN, expected);
    }

    #[test]
    fn ipc_magic_is_four_bytes() {
        assert_eq!(IPC_MAGIC.to_le_bytes().len(), 4);
    }

    #[test]
    fn ipc_magic_does_not_equal_be_encoding() {
        // Verify that the magic value is specifically the little-endian interpretation
        let le_bytes = IPC_MAGIC.to_le_bytes();
        let be_bytes = IPC_MAGIC.to_be_bytes();
        assert_ne!(le_bytes, be_bytes);
    }
}
