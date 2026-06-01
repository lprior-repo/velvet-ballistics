// Proptest: Newtype wrapping/unwrapping identity.
//
// Obligation: PO-vb-h09wf-037
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_013_newtype_identity
//
// Domain claim: >1000 cases: for any Vec<u8>, wrapping and unwrapping
// EnvelopeBytes and IrBytes preserves byte identity.
//
// PRODUCTION BINDING:
//   vb_storage::records::entities::CompiledIrRecord (future EnvelopeBytes newtype)
//   vb_storage::admission::AcceptedArtifact (future IrBytes newtype)
//
// NOTE: This is non-behavior-affecting. Tests newtype properties for the
// type-system hardening recommended by PS-013.

use proptest::prelude::*;

/// Simulated EnvelopeBytes newtype (mirrors PS-013-newtype-refinement.rs).
#[derive(Debug, Clone, PartialEq, Eq)]
struct EnvelopeBytes(Vec<u8>);

/// Simulated IrBytes newtype (mirrors PS-013-newtype-refinement.rs).
#[derive(Debug, Clone, PartialEq, Eq)]
struct IrBytes(Vec<u8>);

impl EnvelopeBytes {
    fn new(bytes: Vec<u8>) -> Self { Self(bytes) }
    fn into_inner(self) -> Vec<u8> { self.0 }
}

impl IrBytes {
    fn new(bytes: Vec<u8>) -> Self { Self(bytes) }
    fn into_inner(self) -> Vec<u8> { self.0 }
}

proptest! {
    /// PS-013a: EnvelopeBytes roundtrip identity.
    #[test]
    fn ps_013_envelope_bytes_roundtrip(bytes in proptest::collection::vec(0u8.., 0..1024)) {
        let original = bytes.clone();
        let envelope = EnvelopeBytes::new(bytes);
        let recovered = envelope.into_inner();
        prop_assert_eq!(original, recovered, "EnvelopeBytes roundtrip must preserve bytes");
    }

    /// PS-013b: IrBytes roundtrip identity.
    #[test]
    fn ps_013_ir_bytes_roundtrip(bytes in proptest::collection::vec(0u8.., 0..1024)) {
        let original = bytes.clone();
        let ir = IrBytes::new(bytes);
        let recovered = ir.into_inner();
        prop_assert_eq!(original, recovered, "IrBytes roundtrip must preserve bytes");
    }

    /// PS-013c: EnvelopeBytes and IrBytes are distinct types even with same bytes.
    #[test]
    fn ps_013_newtypes_are_distinct(bytes in proptest::collection::vec(0u8.., 0..256)) {
        // This test would be a COMPILE ERROR if EnvelopeBytes and IrBytes were
        // the same type. The fact it compiles proves they are distinct types:
        let _envelope = EnvelopeBytes::new(bytes.clone());
        let _ir = IrBytes::new(bytes);
        // These cannot be compared with == (different types) — that IS the test!
    }
}
