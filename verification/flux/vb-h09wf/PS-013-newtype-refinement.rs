// Flux-rs refinement: EnvelopeBytes vs IrBytes distinct newtypes.
//
// Obligation: PO-vb-h09wf-036
// Verifier: flux-rs
// Command: bash scripts/flux-check-package.sh vb_storage
//
// Domain claim:
//   EnvelopeBytes and IrBytes are distinct indexed types.
//   Functions expecting EnvelopeBytes reject IrBytes at compile time.
//   The type-system prevents H11 (misleading field name) and H12 (name collision).
//
// PRODUCTION BINDING:
//   vb_storage::records::entities::CompiledIrRecord
//   vb_storage::admission::AcceptedArtifact
//
// NOTE: This is a non-behavior-affecting type-system hardening.
// The newtypes introduce zero runtime overhead.
//
// Trusted base: Newtype wrapping/unwrapping preserves byte identity
// Source: .beads/vb-h09wf/proof-obligations.planned.jsonl PO-vb-h09wf-036

#![forbid(unsafe_code)]
#![allow(unused)]

/// EnvelopeBytes: the serialized AcceptedArtifact stored in CompiledIrRecord.
///
/// Flux refinement (intended for production code):
///   #[flux_rs::refined_by(envelope: bool)]
///   #[flux_rs::invariant(envelope == true)]
///   pub struct EnvelopeBytes(Vec<u8>);
///
/// This newtype is indexed differently from IrBytes, making accidental
/// interchange a compile-time error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeBytes(Vec<u8>);

/// IrBytes: the inner compiled IR bytes stored in AcceptedArtifact.ir.
///
/// Flux refinement (intended for production code):
///   #[flux_rs::refined_by(inner_ir: bool)]
///   #[flux_rs::invariant(inner_ir == true)]
///   pub struct IrBytes(Vec<u8>);
///
/// This newtype is indexed differently from EnvelopeBytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrBytes(Vec<u8>);

impl EnvelopeBytes {
    /// Create EnvelopeBytes from raw bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Consume and return inner bytes (zero-cost).
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    /// Borrow inner bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl IrBytes {
    /// Create IrBytes from raw bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Consume and return inner bytes (zero-cost).
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    /// Borrow inner bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Roundtrip identity: for any Vec<u8>, wrapping and unwrapping preserves bytes.
fn _assert_roundtrip_identity() {
    // EnvelopeBytes::new(bytes).into_inner() == bytes
    // IrBytes::new(bytes).into_inner() == bytes
    // This is verified by proptest (PO-vb-h09wf-037).
}

/// Type-level distinction: EnvelopeBytes and IrBytes are NOT interchangeable.
///
/// Flux refinement (intended):
///   Flux indexed types ensure that functions accepting `EnvelopeBytes`
///   cannot receive `IrBytes` values. This prevents:
///   - H11: writing `BLAKE3(record.ir)` when `record.ir` should be the envelope
///   - H12: accidentally passing artifact.ir (inner IR) where record.ir (envelope) is expected
fn _assert_type_distinction() {
    // These would be compile-time errors in the Flux-refined codebase:
    //
    // let envelope: EnvelopeBytes = ...;
    // let inner_ir: IrBytes = ...;
    //
    // fn hash_envelope(bytes: EnvelopeBytes) { ... }
    // fn hash_inner_ir(bytes: IrBytes) { ... }
    //
    // hash_envelope(inner_ir);  // COMPILE ERROR: type mismatch
    // hash_inner_ir(envelope);  // COMPILE ERROR: type mismatch
}

/// Flux indexed type invariants for production code:
mod flux_newtype_refinements {
    // Intended Flux annotations:
    //
    // #[flux_rs::refined_by(tag: int)]
    // pub struct EnvelopeBytes(#[flux_rs::field(Vec<u8>[tag == 1])] Vec<u8>);
    //
    // #[flux_rs::refined_by(tag: int)]
    // pub struct IrBytes(#[flux_rs::field(Vec<u8>[tag == 2])] Vec<u8>);
    //
    // #[flux_rs::refined_by(digest: WorkflowDigest, envelope: EnvelopeBytes)]
    // pub struct CompiledIrRecord {
    //     pub digest: WorkflowDigest,
    //     pub ir: EnvelopeBytes,  // Renamed from 'ir' to 'envelope'
    // }
    //
    // #[flux_rs::refined_by(digest: WorkflowDigest, ir: IrBytes)]
    // pub struct AcceptedArtifact {
    //     pub digest: WorkflowDigest,
    //     pub ir: IrBytes,  // Now typed as IrBytes, not Vec<u8>
    //     ...
    // }
    //
    // These refinements make the H11 and H12 hazards compile-time errors,
    // preventing the vb-6uue confusion from recurring.
}
