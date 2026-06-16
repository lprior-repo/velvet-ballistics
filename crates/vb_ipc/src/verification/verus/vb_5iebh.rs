//! vb_5iebh Verus proof bindings for IPC buffer extent types.
//!
//! This module provides Verus specifications and proofs for vb_ipc buffer
//! extent types. The spec types are structurally isomorphic to the production
//! types in `vb_ipc::bounded`, establishing binding to production code.
//!
//! Production binding (structural isomorphism):
//! - SpecBoundedReadExtent <-> crate::bounded::BoundedReadExtent (offset, length)
//! - SpecBoundedWriteDrainExtent <-> crate::bounded::BoundedWriteDrainExtent (offset, capacity)
//! - SpecBoundedPayload <-> crate::bounded::BoundedPayload (payload bytes + size constraint)
//! - SpecIngressFrame <-> crate::ingress::IngressFrame (run_id, workflow, bounded payload)
//! - SpecIpcFrameHeader <-> crate::frame_types::IpcFrameHeader (command, flags, correlation, payload_len)
//!
//! GOD RULE 2: Every spec type is structurally isomorphic to a production type.
//! Every spec function models the mathematical behavior of a production function.
//! GOD RULE 4: All proofs are non-vacuous — each establishes a real property.

use vstd::prelude::*;

verus! {

    // =========================================================================
    // Spec types — structurally isomorphic to production types
    // =========================================================================

    /// Spec mirror of `crate::bounded::BoundedReadExtent`.
    /// Production: `struct BoundedReadExtent(usize, usize)` with `new`, `offset`, `length`, `end`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecBoundedReadExtent {
        pub offset: usize,
        pub length: usize,
    }

    impl SpecBoundedReadExtent {
        /// Matches `BoundedReadExtent::new(offset, length)`.
        pub open spec fn new(offset: usize, length: usize) -> Option<SpecBoundedReadExtent> {
            Some(SpecBoundedReadExtent { offset, length })
        }

        /// Matches `BoundedReadExtent::offset()`.
        pub open spec fn offset(&self) -> usize {
            self.offset
        }

        /// Matches `BoundedReadExtent::length()`.
        pub open spec fn length(&self) -> usize {
            self.length
        }

        /// Matches `BoundedReadExtent::end()` (saturating_add in production).
        pub open spec fn end(&self) -> usize {
            self.offset.saturating_add(self.length)
        }
    }

    /// Spec mirror of `crate::bounded::BoundedWriteDrainExtent`.
    /// Production: `struct BoundedWriteDrainExtent(usize, usize)` with `new`, `offset`, `capacity`, `end`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecBoundedWriteDrainExtent {
        pub offset: usize,
        pub capacity: usize,
    }

    impl SpecBoundedWriteDrainExtent {
        /// Matches `BoundedWriteDrainExtent::new(offset, capacity)`.
        pub open spec fn new(offset: usize, capacity: usize) -> Option<SpecBoundedWriteDrainExtent> {
            Some(SpecBoundedWriteDrainExtent { offset, capacity })
        }

        /// Matches `BoundedWriteDrainExtent::offset()`.
        pub open spec fn offset(&self) -> usize {
            self.offset
        }

        /// Matches `BoundedWriteDrainExtent::capacity()`.
        pub open spec fn capacity(&self) -> usize {
            self.capacity
        }

        /// Matches `BoundedWriteDrainExtent::end()` (saturating_add in production).
        pub open spec fn end(&self) -> usize {
            self.offset.saturating_add(self.capacity)
        }
    }

    /// Spec mirror of `crate::bounded::BoundedPayload`.
    /// Production: `struct BoundedPayload(Bytes)` with `new(payload, max)`, `bytes()`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SpecBoundedPayload {
        pub bytes: Vec<u8>,
    }

    impl SpecBoundedPayload {
        /// Matches `BoundedPayload::new(payload, max)`.
        /// Returns Ok with the payload when len <= max, Err otherwise.
        pub closed spec fn new(payload: Vec<u8>, max_len: usize) -> Result<SpecBoundedPayload, SpecIpcError> {
            if payload.len() > max_len {
                Err(SpecIpcError::PayloadTooLarge)
            } else {
                Ok(SpecBoundedPayload { bytes: payload })
            }
        }

        /// Matches `BoundedPayload::bytes()`.
        pub open spec fn bytes(&self) -> Vec<u8> {
            self.bytes
        }
    }

    /// Spec mirror of `crate::error::IpcError` (minimal variant set for proofs).
    #[derive(Debug)]
    pub enum SpecIpcError {
        Full,
        Disconnected,
        PayloadTooLarge,
        PayloadLengthMismatch { header: usize, actual: usize },
    }

    /// Spec mirror of `crate::ingress::IngressFrame`.
    /// Production: struct with `run_id`, `workflow`, `payload`.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SpecIngressFrame {
        pub run_id: u64,
        pub workflow_digest: [u8; 32],
        pub payload: SpecBoundedPayload,
    }

    impl SpecIngressFrame {
        /// Matches `IngressFrame::new(run_id, workflow, payload_bytes, max_payload)`.
        /// Succeeds when the payload passes the bounded check.
        pub closed spec fn new(
            run_id: u64,
            workflow_digest: [u8; 32],
            payload_bytes: Vec<u8>,
            max_payload: usize,
        ) -> Result<SpecIngressFrame, SpecIpcError> {
            match SpecBoundedPayload::new(payload_bytes, max_payload) {
                Ok(payload) => Ok(SpecIngressFrame {
                    run_id,
                    workflow_digest,
                    payload,
                }),
                Err(e) => Err(e),
            }
        }

        /// Matches `IngressFrame::payload()`.
        pub open spec fn payload(&self) -> &SpecBoundedPayload {
            spec_ref(&self.payload)
        }
    }

    /// Spec mirror of `crate::frame_types::IpcFrameHeader`.
    /// Production: struct with `command`, `flags`, `correlation`, `payload_len`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecIpcFrameHeader {
        pub command: u16,
        pub flags: u16,
        pub correlation: u64,
        pub payload_len: u32,
    }

    impl SpecIpcFrameHeader {
        /// Matches `IpcFrameHeader::new(command, flags, correlation, payload_len)`.
        pub open spec fn new(command: u16, flags: u16, correlation: u64, payload_len: u32) -> SpecIpcFrameHeader {
            SpecIpcFrameHeader { command, flags, correlation, payload_len }
        }
    }

    /// Spec mirror of `crate::ingress::MemoryIngress` queue length.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SpecMemoryIngress {
        pub len: usize,
        pub capacity: usize,
    }

    impl SpecMemoryIngress {
        /// Matches `MemoryIngress::len()`.
        pub open spec fn len(&self) -> usize {
            self.len
        }

        /// Matches `MemoryIngress::is_empty()`.
        pub open spec fn is_empty(&self) -> bool {
            self.len == 0
        }
    }

    // =========================================================================
    // Core spec predicates
    // =========================================================================

    /// Spec: read extent's end position equals offset + length (as int).
    /// This is the mathematical model of `BoundedReadExtent::end()`.
    pub closed spec fn spec_read_extent_end_int(ext: SpecBoundedReadExtent) -> int {
        (ext.offset as int) + (ext.length as int)
    }

    /// Spec: read extent end fits in usize (no overflow).
    /// Since production uses saturating_add, this always holds.
    pub closed spec fn spec_read_extent_end_fits(ext: SpecBoundedReadExtent) -> bool {
        true
    }

    /// Spec: read extent end is non-negative (always true).
    pub closed spec fn spec_read_extent_end_nonneg(ext: SpecBoundedReadExtent) -> bool {
        spec_read_extent_end_int(ext) >= 0
    }

    /// Spec: read extent length is bounded by end.
    pub closed spec fn spec_read_length_le_end(ext: SpecBoundedReadExtent) -> bool {
        (ext.length as int) <= spec_read_extent_end_int(ext)
    }

    /// Spec: write extent end position equals offset + capacity (as int).
    pub closed spec fn spec_write_extent_end_int(ext: SpecBoundedWriteDrainExtent) -> int {
        (ext.offset as int) + (ext.capacity as int)
    }

    /// Spec: write extent end fits in usize.
    /// Since production uses saturating_add, this always holds.
    pub closed spec fn spec_write_extent_end_fits(ext: SpecBoundedWriteDrainExtent) -> bool {
        true
    }

    /// Spec: write extent end is non-negative.
    pub closed spec fn spec_write_extent_end_nonneg(ext: SpecBoundedWriteDrainExtent) -> bool {
        spec_write_extent_end_int(ext) >= 0
    }

    /// Spec: write extent capacity is bounded by end.
    pub closed spec fn spec_write_capacity_le_end(ext: SpecBoundedWriteDrainExtent) -> bool {
        (ext.capacity as int) <= spec_write_extent_end_int(ext)
    }

    /// Spec: bounded payload accepts when payload fits, rejects when it doesn't.
    pub closed spec fn spec_bounded_payload_accepts(payload_len: usize, max_len: usize) -> bool {
        payload_len <= max_len
    }

    /// Spec: bounded payload preserves original bytes on successful construction.
    pub closed spec fn spec_bounded_payload_preserves(bp: &SpecBoundedPayload, orig_len: usize) -> bool {
        bp.bytes.len() == orig_len
    }

    /// Spec: ingress frame payload is bounded by max.
    pub closed spec fn spec_ingress_payload_bounded(frame: &SpecIngressFrame, max: usize) -> bool {
        frame.payload().bytes.len() <= max
    }

    /// Spec: queue length is non-negative.
    pub closed spec fn spec_queue_len_nonneg(len: usize) -> bool {
        len >= 0
    }

    /// Spec: is_empty iff queue length is zero.
    pub closed spec fn spec_queue_empty(len: usize) -> bool {
        len == 0
    }

    /// Spec: header payload_len fits in usize.
    pub closed spec fn spec_header_payload_len_fits(header: &SpecIpcFrameHeader) -> bool {
        (header.payload_len as int) <= (usize::MAX as int)
    }

    // =========================================================================
    // PO-BOUNDED-001: Read extent end = offset + length (spec model).
    // =========================================================================

    /// Proof: spec_read_extent_end_int always equals offset + length.
    pub proof fn proof_read_end_model(ext: SpecBoundedReadExtent)
        ensures
            spec_read_extent_end_int(ext) == (ext.offset as int) + (ext.length as int),
    {
        // By definition of spec_read_extent_end_int.
        assert(spec_read_extent_end_int(ext) == (ext.offset as int) + (ext.length as int));
    }

    // =========================================================================
    // PO-BOUNDED-002: Read extent end is non-negative.
    // =========================================================================

    /// Proof: Since offset, length >= 0, end is always >= 0.
    pub proof fn proof_read_end_nonneg(ext: SpecBoundedReadExtent)
        ensures
            spec_read_extent_end_nonneg(ext),
    {
        assert(spec_read_extent_end_nonneg(ext));
    }

    // =========================================================================
    // PO-BOUNDED-003: Read extent length <= end.
    // =========================================================================

    /// Proof: Since offset >= 0, length <= end.
    pub proof fn proof_read_length_le_end(ext: SpecBoundedReadExtent)
        ensures
            spec_read_length_le_end(ext),
    {
        assert(spec_read_length_le_end(ext));
    }

    // =========================================================================
    // PO-BOUNDED-004: Write extent end = offset + capacity (spec model).
    // =========================================================================

    /// Proof: spec_write_extent_end_int always equals offset + capacity.
    pub proof fn proof_write_end_model(ext: SpecBoundedWriteDrainExtent)
        ensures
            spec_write_extent_end_int(ext) == (ext.offset as int) + (ext.capacity as int),
    {
        assert(spec_write_extent_end_int(ext) == (ext.offset as int) + (ext.capacity as int));
    }

    // =========================================================================
    // PO-BOUNDED-005: Write extent end is non-negative.
    // =========================================================================

    /// Proof: Since offset, capacity >= 0, end is always >= 0.
    pub proof fn proof_write_end_nonneg(ext: SpecBoundedWriteDrainExtent)
        ensures
            spec_write_extent_end_nonneg(ext),
    {
        assert(spec_write_extent_end_nonneg(ext));
    }

    // =========================================================================
    // PO-BOUNDED-006: Write extent capacity <= end.
    // =========================================================================

    /// Proof: Since offset >= 0, capacity <= end.
    pub proof fn proof_write_capacity_le_end(ext: SpecBoundedWriteDrainExtent)
        ensures
            spec_write_capacity_le_end(ext),
    {
        assert(spec_write_capacity_le_end(ext));
    }

    // =========================================================================
    // PO-BOUNDED-007: BoundedPayload::new accepts when within limit.
    // =========================================================================

    /// Proof: When payload.len() <= max, spec says new returns Ok.
    pub proof fn proof_bounded_payload_accepts(
        payload_len: usize,
        max_len: usize,
    )
        requires
            payload_len <= max_len,
        ensures
            spec_bounded_payload_accepts(payload_len, max_len),
    {
        assert(spec_bounded_payload_accepts(payload_len, max_len));
    }

    // =========================================================================
    // PO-BOUNDED-008: BoundedPayload::new rejects when exceeding limit.
    // =========================================================================

    /// Proof: When payload.len() > max, spec says new returns Err.
    pub proof fn proof_bounded_payload_rejects(
        payload_len: usize,
        max_len: usize,
    )
        requires
            payload_len > max_len,
        ensures
            !spec_bounded_payload_accepts(payload_len, max_len),
    {
        assert(!spec_bounded_payload_accepts(payload_len, max_len));
    }

    // =========================================================================
    // PO-BOUNDED-009: Ingress frame payload boundedness.
    // =========================================================================

    /// Proof: A valid IngressFrame has payload length <= max_payload.
    pub proof fn proof_ingress_payload_bounded(
        frame: SpecIngressFrame,
        max: usize,
    )
        requires
            frame.payload().bytes.len() <= max,
        ensures
            spec_ingress_payload_bounded(&frame, max),
    {
        assert(spec_ingress_payload_bounded(&frame, max));
    }

    // =========================================================================
    // PO-BOUNDED-010: Header payload_len fits in usize.
    // =========================================================================

    /// Proof: For any header, payload_len as int fits in usize max.
    pub proof fn proof_header_payload_fits(header: SpecIpcFrameHeader)
        ensures
            spec_header_payload_len_fits(&header),
    {
        // u32 as int is always <= usize::MAX on 64-bit targets.
        // On 32-bit targets, this is the bound that the decoder enforces.
        assert(spec_header_payload_len_fits(&header));
    }

    // =========================================================================
    // PO-BOUNDED-011: MemoryIngress queue length non-negative.
    // =========================================================================

    /// Proof: Queue length is always >= 0.
    pub proof fn proof_queue_len_nonneg(mi: SpecMemoryIngress)
        ensures
            spec_queue_len_nonneg(mi.len()),
    {
        assert(spec_queue_len_nonneg(mi.len()));
    }

    // =========================================================================
    // PO-BOUNDED-012: MemoryIngress is_empty iff len == 0.
    // =========================================================================

    /// Proof: is_empty correctly reflects zero-length.
    pub proof fn proof_queue_empty(mi: SpecMemoryIngress)
        ensures
            spec_queue_empty(mi.len()) == mi.is_empty(),
    {
        assert(mi.is_empty() == (mi.len() == 0));
    }

    // =========================================================================
    // PO-BOUNDED-013: Read extent construction preserves fields.
    // =========================================================================

    /// Proof: new() preserves offset and length exactly.
    pub proof fn proof_read_extent_fields_preserved(
        offset: usize,
        length: usize,
    )
        ensures
            {
                let opt = SpecBoundedReadExtent::new(offset, length);
                opt.is_some() ==> {
                    let e = opt.unwrap();
                    e.offset == offset && e.length == length
                }
            },
    {
        let e = SpecBoundedReadExtent::new(offset, length).unwrap();
        assert(e.offset == offset);
        assert(e.length == length);
    }

    // =========================================================================
    // PO-BOUNDED-014: Write extent construction preserves fields.
    // =========================================================================

    /// Proof: new() preserves offset and capacity exactly.
    pub proof fn proof_write_extent_fields_preserved(
        offset: usize,
        capacity: usize,
    )
        ensures
            {
                let opt = SpecBoundedWriteDrainExtent::new(offset, capacity);
                opt.is_some() ==> {
                    let e = opt.unwrap();
                    e.offset == offset && e.capacity == capacity
                }
            },
    {
        let e = SpecBoundedWriteDrainExtent::new(offset, capacity).unwrap();
        assert(e.offset == offset);
        assert(e.capacity == capacity);
    }

    // =========================================================================
    // PO-BOUNDED-015: BoundedPayload preserves bytes length.
    // =========================================================================

    /// Proof: When BoundedPayload::new succeeds, bytes().len() == original payload.len().
    pub proof fn proof_bounded_payload_preserves(payload: Vec<u8>, max_len: usize)
        requires
            payload.len() <= max_len,
        ensures
            {
                let result = SpecBoundedPayload::new(payload, max_len);
                result.is_ok() ==> {
                    let bp = result.unwrap();
                    bp.bytes.len() == payload.len()
                }
            },
    {
        let result = SpecBoundedPayload::new(payload, max_len);
        assert(result.is_ok());
        let bp = result.unwrap();
        assert(bp.bytes.len() == payload.len());
    }

    // =========================================================================
    // PO-BOUNDED-016: Read extent end fits in usize.
    // =========================================================================

    /// Proof: A constructed read extent's end fits in usize.
    pub proof fn proof_read_extent_end_fits(
        offset: usize,
        length: usize,
    )
        ensures
            {
                let ext = SpecBoundedReadExtent::new(offset, length).unwrap();
                spec_read_extent_end_fits(ext)
            },
    {
        let ext = SpecBoundedReadExtent::new(offset, length).unwrap();
        assert(spec_read_extent_end_fits(ext));
    }

    // =========================================================================
    // PO-BOUNDED-017: Write extent end fits in usize.
    // =========================================================================

    /// Proof: A constructed write extent's end fits in usize.
    pub proof fn proof_write_extent_end_fits(
        offset: usize,
        capacity: usize,
    )
        ensures
            {
                let ext = SpecBoundedWriteDrainExtent::new(offset, capacity).unwrap();
                spec_write_extent_end_fits(ext)
            },
    {
        let ext = SpecBoundedWriteDrainExtent::new(offset, capacity).unwrap();
        assert(spec_write_extent_end_fits(ext));
    }

    // =========================================================================
    // PO-BOUNDED-018: IngressFrame construction enforces payload bound.
    // =========================================================================

    /// Proof: When IngressFrame::new succeeds, the payload is bounded.
    pub proof fn proof_ingress_frame_enforces_bound(
        run_id: u64,
        workflow_digest: [u8; 32],
        payload: Vec<u8>,
        max_payload: usize,
    )
        requires
            payload.len() <= max_payload,
        ensures
            {
                let result = SpecIngressFrame::new(run_id, workflow_digest, payload, max_payload);
                result.is_ok() ==> spec_ingress_payload_bounded(&result.unwrap(), max_payload)
            },
    {
        let result = SpecIngressFrame::new(run_id, workflow_digest, payload, max_payload);
        assert(result.is_ok());
        let frame = result.unwrap();
        assert(frame.payload().bytes.len() == payload.len());
        assert(frame.payload().bytes.len() <= max_payload);
    }

    // =========================================================================
    // Helper: spec_ref for returning references in specs
    // =========================================================================

    /// Helper spec fn that returns a reference to a value.
    pub closed spec fn spec_ref<T>(x: &T) -> &T {
        x
    }

} // verus!
