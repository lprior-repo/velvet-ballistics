// vb_validate Proptest Invariants — RED PHASE
// Property-based tests for validate_gate_08 and pipeline validate invariants.

#![forbid(unsafe_code)]

use crate::gates::validate_gate_08_accessor_path_segments;
use proptest::prelude::*;
use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
use vb_core::span::Span;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helper: WorkflowParts factory
// ---------------------------------------------------------------------------

fn arb_parts(
    slot_count: u16,
    symbols_count: u32,
    accessors: Vec<AccessorProgram>,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("prop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors: accessors.into_boxed_slice(),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn arb_accessor(root: u16, path: Vec<PathSegment>) -> AccessorProgram {
    AccessorProgram {
        root: SlotIdx::new(root),
        path: path.into_boxed_slice(),
    }
}

// ---------------------------------------------------------------------------
// INV-01: Symbol bounds — validate_gate_08
//
// For any accessor with a Field segment:
// - If symbol < symbols_count → validate returns Ok
// - If symbol >= symbols_count → validate returns Err(AccessorSymbolOutOfBounds)
// ---------------------------------------------------------------------------

proptest! {
    /// When symbol < symbols_count, gate_08 must return Ok.
    #[test]
    fn gate_08_symbol_in_bounds_returns_ok(
        // Use prop_filter to ensure symbol < symbols_count
        symbol in 0u32..100u32,
        symbols_count in 1u32..200u32,
    ) {
        prop_assume!(symbol < symbols_count, "symbol must be < symbols_count for this test");
        let sym_id = SymbolId::new(symbol);
        let accessor = arb_accessor(0, vec![PathSegment::Field(sym_id)]);
        let parts = arb_parts(1, symbols_count, vec![accessor]);

        let result = validate_gate_08_accessor_path_segments(&parts);

        prop_assert!(
            result.is_ok(),
            "validate_gate_08 should pass when symbol {symbol} < symbols_count {symbols_count}, got {result:?}"
        );
    }

    /// When symbol >= symbols_count, gate_08 must return Err(AccessorSymbolOutOfBounds).
    #[test]
    fn gate_08_symbol_out_of_bounds_returns_err(
        symbol in 100u32..500u32,
        symbols_count in 1u32..99u32,
    ) {
        // symbol is always >= 100 > symbols_count (which is < 99), so symbol >= symbols_count always holds
        prop_assume!(symbol >= symbols_count);
        let sym_id = SymbolId::new(symbol);
        let accessor = arb_accessor(0, vec![PathSegment::Field(sym_id)]);
        let parts = arb_parts(1, symbols_count, vec![accessor]);

        let result = validate_gate_08_accessor_path_segments(&parts);

        prop_assert!(
            result.is_err(),
            "validate_gate_08 should reject when symbol {symbol} >= symbols_count {symbols_count}, got {result:?}"
        );

        let err = result.unwrap_err();
        prop_assert!(
            matches!(
                &err,
                crate::ValidationError::AccessorSymbolOutOfBounds {
                    accessor_index: 0,
                    segment_index: 0,
                    symbol: actual_symbol,
                    symbols_count: actual_symbols_count,
                 span: Span::ZERO} if *actual_symbol == symbol && *actual_symbols_count == symbols_count
            ),
            "wrong error variant or field values: got {err:?}"
        );
    }

    /// Mixed accessors: first valid, second invalid → error is at index 1.
    #[test]
    fn gate_08_mixed_accessors_error_at_second(
        root0 in 0u16..3u16,
        sym0 in 0u32..3u32,
        root1 in 0u16..3u16,
        sym1 in 50u32..200u32,
        symbols_count in 1u32..10u32,
    ) {
        // Ensure first accessor is always valid: sym0 < symbols_count
        prop_assume!(sym0 < symbols_count);
        // Second accessor is always invalid: sym1 >= 50 > symbols_count
        prop_assume!(sym1 >= symbols_count);

        let accessor0 = arb_accessor(root0, vec![PathSegment::Field(SymbolId::new(sym0))]);
        let accessor1 = arb_accessor(root1, vec![PathSegment::Field(SymbolId::new(sym1))]);
        let parts = arb_parts(10, symbols_count, vec![accessor0, accessor1]);

        let result = validate_gate_08_accessor_path_segments(&parts);

        prop_assert!(
            result.is_err(),
            "expected Err for out-of-bounds second accessor, got {result:?}"
        );
        let err = result.unwrap_err();
        prop_assert!(
            matches!(
                &err,
                crate::ValidationError::AccessorSymbolOutOfBounds {
                    accessor_index: 1,
                    segment_index: 0,
                    symbol: actual_symbol,
                    symbols_count: actual_symbols_count,
                 span: Span::ZERO} if *actual_symbol == sym1 && *actual_symbols_count == symbols_count
            ),
            "expected error at accessor_index=1 with symbol={sym1}, symbols_count={symbols_count}, got {err:?}"
        );
    }

    /// Empty accessors always pass gate 8.
    #[test]
    fn gate_08_empty_accessors_returns_ok(symbols_count in 0u32..100u32) {
        let parts = arb_parts(1, symbols_count, vec![]);
        prop_assert!(
            validate_gate_08_accessor_path_segments(&parts).is_ok(),
            "empty accessors should always pass gate 8"
        );
    }

    /// Index segments do not trigger symbol bounds errors (only Field segments do).
    #[test]
    fn gate_08_index_segments_do_not_cause_symbol_error(
        root in 0u16..5u16,
        idx0 in 0u32..u32::MAX,
        symbols_count in 0u32..50u32,
    ) {
        let accessor = arb_accessor(root, vec![PathSegment::Index(idx0)]);
        let parts = arb_parts(10, symbols_count, vec![accessor]);

        let result = validate_gate_08_accessor_path_segments(&parts);

        // Index segments should never produce AccessorSymbolOutOfBounds
        if idx0 != u32::MAX {
            // Normal index: should pass or fail for non-symbol reasons
            if result.is_err() {
                let err = result.unwrap_err();
                prop_assert!(
                    !matches!(err, crate::ValidationError::AccessorSymbolOutOfBounds { .. }),
                    "Index segment should not cause AccessorSymbolOutOfBounds, got {err:?}"
                );
            }
        } else {
            // Sentinel index: should fail with AccessorPathInvalid, not symbol error
            prop_assert!(
                matches!(result, Err(crate::ValidationError::AccessorPathInvalid { .. })),
                "Sentinel u32::MAX index should produce AccessorPathInvalid, got {result:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// INV-02: Pipeline determinism
//
// validate(parts) must be deterministic: same input → same output
// ---------------------------------------------------------------------------

proptest! {
    /// validate must be deterministic across multiple calls.
    #[test]
    fn pipeline_validate_is_deterministic(
        slot_count in 1u16..10u16,
        symbols_count in 0u32..20u32,
        sym0 in 0u32..10u32,
        sym1 in 0u32..10u32,
    ) {
        let accessors = vec![
            arb_accessor(0, vec![PathSegment::Field(SymbolId::new(sym0))]),
            arb_accessor(1, vec![PathSegment::Field(SymbolId::new(sym1))]),
        ];
        let parts = arb_parts(slot_count, symbols_count, accessors);

        let r1 = crate::shared::validate(&parts);
        let r2 = crate::shared::validate(&parts);
        let r3 = crate::shared::validate(&parts);

        // Determinism: same inputs produce same output
        prop_assert_eq!(r1.clone(), r2.clone());
        prop_assert_eq!(r2, r3);
    }
}

// ---------------------------------------------------------------------------
// INV-03: Error ordering invariant
//
// Gate 8 checks root first, then symbol. If root is invalid, AccessorSlotOutOfRange
// fires before AccessorSymbolOutOfBounds.
// ---------------------------------------------------------------------------

proptest! {
    /// When both accessor root and symbol are invalid, root check fires first.
    #[test]
    fn gate_08_root_check_before_symbol_check(
        acc_root in 10u16..20u16, // definitely >= slot_count (which is 1)
        acc_sym in 100u32..200u32, // definitely >= symbols_count
        symbols_count in 1u32..10u32,
    ) {
        let accessor = arb_accessor(acc_root, vec![PathSegment::Field(SymbolId::new(acc_sym))]);
        let parts = arb_parts(1, symbols_count, vec![accessor]);

        let result = validate_gate_08_accessor_path_segments(&parts);

        prop_assert!(result.is_err(), "invalid accessor should produce error");
        let err = result.unwrap_err();

        // Root check (SlotOutOfRange) fires before symbol check
        prop_assert!(
            matches!(&err, crate::ValidationError::AccessorSlotOutOfRange { .. }),
            "expected AccessorSlotOutOfRange (root checked first), got {err:?}"
        );
    }
}
