//! Local Verus sanity model for selected `vb_ipc` wire-format constants.
//!
//! **Status:** retired as production proof evidence by bead `vb-dzibx`.
//!
//! This file is intentionally **not** a deductive proof of production Rust
//! behavior.  It contains no `extern_spec!`, no `assume_specification`, and no
//! production `requires`/`ensures` binding for `crate::bounded`,
//! `crate::frame_types`, `crate::ingress`, or `crate::codec` functions.  The
//! previous version claimed structural isomorphism to production types while
//! using divergent mirror types and wrong wire constants; those claims and the
//! tautological roundtrip lemma have been removed.
//!
//! Retained checks are local-model sanity checks only:
//! - IPC magic is the production `VBLT` value (`0x5642_4C54`).
//! - IPC fixed header length is 24 bytes.
//! - The local error enum names every currently present `IpcError` variant.
//! - Header construction uses a local command enum instead of a raw command id.

use vstd::prelude::*;

verus! {

    // =========================================================================
    // Local constants mirroring the documented production wire values.
    // =========================================================================

    pub closed spec fn spec_ipc_magic() -> u32 {
        0x5642_4C54u32
    }

    pub closed spec fn spec_ipc_version() -> u16 {
        1u16
    }

    pub closed spec fn spec_ipc_header_len() -> usize {
        24usize
    }

    pub closed spec fn spec_ipc_header_layout_width() -> nat {
        4 + 2 + 2 + 2 + 2 + 8 + 4
    }

    // =========================================================================
    // Local analogues only.  No production binding or isomorphism is claimed.
    // =========================================================================

    /// Local analogue of `crate::commands::IpcCommand`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SpecIpcCommand {
        SubmitRun,
        SubmitRunInline,
        CancelRun,
        InspectRun,
        ListEvents,
        AnswerAsk,
        CompleteAction,
        FailAction,
        DrainTrace,
        Health,
        Shutdown,
        UnknownCommand(u16),
    }

    /// Local analogue of `crate::bounded::MaxPayloadBytes`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecMaxPayloadBytes {
        pub value: usize,
    }

    impl SpecMaxPayloadBytes {
        pub open spec fn valid(self) -> bool {
            self.value > 0
        }

        pub open spec fn get(self) -> usize {
            self.value
        }
    }

    /// Local analogue of `crate::frame_types::IpcFrameHeader`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecIpcFrameHeader {
        pub command: SpecIpcCommand,
        pub flags: u16,
        pub correlation: u64,
        pub payload_len: u32,
    }

    impl SpecIpcFrameHeader {
        pub open spec fn new(
            command: SpecIpcCommand,
            flags: u16,
            correlation: u64,
            payload_len: u32,
        ) -> SpecIpcFrameHeader {
            SpecIpcFrameHeader { command, flags, correlation, payload_len }
        }
    }

    /// Local analogue of every currently present `crate::error::IpcError` variant.
    #[allow(inconsistent_fields)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SpecIpcError {
        Full,
        Disconnected,
        PayloadTooLarge { actual: usize, limit: usize },
        InvalidMagic { actual: u32 },
        UnsupportedVersion { actual: u16 },
        UnknownCommand(u16),
        ReservedNonZero { actual: u16 },
        PayloadLengthMismatch { header: usize, actual: usize },
        HeaderEncodeFailed,
        HeaderDecodeFailed,
        PayloadLengthOutOfRange { actual: u32 },
        PayloadEncodeFailed,
        PayloadDecodeFailed,
        ResponseDecodeFailed,
    }

    pub closed spec fn spec_ipc_error_variant_count() -> nat {
        14
    }

    pub closed spec fn spec_payload_len_fits_usize(payload_len: u32) -> bool {
        (payload_len as int) <= (usize::MAX as int)
    }

    pub closed spec fn spec_payload_within_max(
        payload_len: u32,
        max_payload: SpecMaxPayloadBytes,
    ) -> bool {
        (payload_len as int) <= (max_payload.get() as int)
    }

    pub closed spec fn spec_decode_fixed_fields_accept(
        magic: u32,
        version: u16,
        reserved: u16,
    ) -> bool {
        magic == spec_ipc_magic() && version == spec_ipc_version() && reserved == 0u16
    }

    pub closed spec fn spec_decode_accepts_valid_header_shape(
        magic: u32,
        version: u16,
        reserved: u16,
        payload_len: u32,
        max_payload: SpecMaxPayloadBytes,
    ) -> bool {
        spec_decode_fixed_fields_accept(magic, version, reserved)
            && spec_payload_len_fits_usize(payload_len)
            && spec_payload_within_max(payload_len, max_payload)
    }

    pub closed spec fn spec_frame_length_agrees(
        header_payload_len: u32,
        actual_payload_len: usize,
    ) -> bool {
        (header_payload_len as int) == (actual_payload_len as int)
    }

    pub closed spec fn spec_end_to_end_bounded(
        payload_len: usize,
        max_payload: SpecMaxPayloadBytes,
        header_payload_len: u32,
    ) -> bool {
        (payload_len as int) <= (max_payload.get() as int)
            && spec_frame_length_agrees(header_payload_len, payload_len)
    }

    // =========================================================================
    // Local-model sanity lemmas.  These are not production proof evidence.
    // =========================================================================

    pub proof fn local_proof_constants_match_vblt_header_layout()
        ensures
            spec_ipc_magic() == 0x5642_4C54u32,
            spec_ipc_version() == 1u16,
            spec_ipc_header_len() == 24usize,
            spec_ipc_header_layout_width() == 24,
            spec_ipc_error_variant_count() == 14,
    {
        assert(spec_ipc_magic() == 0x5642_4C54u32);
        assert(spec_ipc_version() == 1u16);
        assert(spec_ipc_header_len() == 24usize);
        assert(spec_ipc_header_layout_width() == 24);
        assert(spec_ipc_error_variant_count() == 14);
    }

    pub proof fn local_proof_header_new_preserves_fields(
        command: SpecIpcCommand,
        flags: u16,
        correlation: u64,
        payload_len: u32,
    )
        ensures
            SpecIpcFrameHeader::new(command, flags, correlation, payload_len).command == command,
            SpecIpcFrameHeader::new(command, flags, correlation, payload_len).flags == flags,
            SpecIpcFrameHeader::new(command, flags, correlation, payload_len).correlation == correlation,
            SpecIpcFrameHeader::new(command, flags, correlation, payload_len).payload_len == payload_len,
    {
        let header = SpecIpcFrameHeader::new(command, flags, correlation, payload_len);
        assert(header.command == command);
        assert(header.flags == flags);
        assert(header.correlation == correlation);
        assert(header.payload_len == payload_len);
    }

    pub proof fn local_proof_decode_accepts_valid_header_shape(
        magic: u32,
        version: u16,
        reserved: u16,
        payload_len: u32,
        max_payload: SpecMaxPayloadBytes,
    )
        requires
            magic == spec_ipc_magic(),
            version == spec_ipc_version(),
            reserved == 0u16,
            spec_payload_len_fits_usize(payload_len),
            spec_payload_within_max(payload_len, max_payload),
        ensures
            spec_decode_accepts_valid_header_shape(
                magic,
                version,
                reserved,
                payload_len,
                max_payload,
            ),
    {
        assert(spec_decode_fixed_fields_accept(magic, version, reserved));
        assert(spec_payload_len_fits_usize(payload_len));
        assert(spec_payload_within_max(payload_len, max_payload));
        assert(spec_decode_accepts_valid_header_shape(
            magic,
            version,
            reserved,
            payload_len,
            max_payload,
        ));
    }

    pub proof fn local_proof_frame_length_agreement(
        header_payload_len: u32,
        actual_payload_len: usize,
    )
        requires
            (header_payload_len as int) == (actual_payload_len as int),
        ensures
            spec_frame_length_agrees(header_payload_len, actual_payload_len),
    {
        assert(spec_frame_length_agrees(header_payload_len, actual_payload_len));
    }

    pub proof fn local_proof_end_to_end_bounded(
        payload_len: usize,
        max_payload: SpecMaxPayloadBytes,
        header_payload_len: u32,
    )
        requires
            (payload_len as int) <= (max_payload.get() as int),
            spec_frame_length_agrees(header_payload_len, payload_len),
        ensures
            spec_end_to_end_bounded(payload_len, max_payload, header_payload_len),
    {
        assert((payload_len as int) <= (max_payload.get() as int));
        assert(spec_frame_length_agrees(header_payload_len, payload_len));
        assert(spec_end_to_end_bounded(payload_len, max_payload, header_payload_len));
    }

} // verus!
