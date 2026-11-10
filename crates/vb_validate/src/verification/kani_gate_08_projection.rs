#![cfg(kani)]
#![forbid(unsafe_code)]
//! Canonical proofs over exactly the production Gate 8 projection.
//!
//! The public gate delegates to this private helper with these three fields.
//! Unrelated WorkflowParts fields are not generated or verified here.

use super::validate_accessor_paths;
use crate::ValidationError;
use vb_core::ids::{SlotIdx, SymbolId};
use vb_core::workflow::{AccessorProgram, PathSegment};

#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_valid_zero_accessors_pass() {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    kani::assert(
        validate_accessor_paths(&[], slot_count, symbols_count).is_ok(),
        "zero accessors pass Gate 8",
    );
    kani::cover(
        slot_count == 0 && symbols_count == 0,
        "original empty projection",
    );
    kani::cover(
        slot_count == u16::MAX && symbols_count == u32::MAX,
        "maximum counts",
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_valid_accessors_pass() {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root: u16 = kani::any();
    let symbol: u32 = kani::any();
    let index: u32 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 16);
    kani::assume(symbols_count > 0);
    kani::assume(symbols_count <= 64);
    kani::assume(root < slot_count);
    kani::assume(symbol < symbols_count);
    kani::assume(index != u32::MAX);
    let shape: u8 = kani::any();
    let accessors: Box<[AccessorProgram]> = match shape {
        0 => Box::new([]),
        1 => Box::new([AccessorProgram {
            root: SlotIdx::new(root),
            path: Box::new([PathSegment::Field(SymbolId::new(symbol))]),
        }]),
        2 => Box::new([AccessorProgram {
            root: SlotIdx::new(root),
            path: Box::new([PathSegment::Index(index)]),
        }]),
        _ => Box::new([AccessorProgram {
            root: SlotIdx::new(root),
            path: Box::new([
                PathSegment::Field(SymbolId::new(symbol)),
                PathSegment::Index(index),
            ]),
        }]),
    };
    kani::assert(
        validate_accessor_paths(&accessors, slot_count, symbols_count).is_ok(),
        "arbitrary valid accessors pass Gate 8",
    );
    kani::cover(shape == 0, "empty accessor table");
    kani::cover(shape == 1, "field path");
    kani::cover(shape == 2, "index path");
    kani::cover(shape > 2, "mixed path");
    kani::cover(
        slot_count == 16 && symbols_count == 64,
        "original count bounds",
    );
    std::mem::forget(accessors);
}

#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_root_oob_rejected() {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 8);
    let accessors = [AccessorProgram {
        root: SlotIdx::new(slot_count),
        path: Box::new([]),
    }];
    kani::assert(
        matches!(validate_accessor_paths(&accessors, slot_count, symbols_count),
            Err(ValidationError::AccessorSlotOutOfRange { accessor_index, slot, slot_count: actual_count })
                if accessor_index == 0 && slot == usize::from(slot_count) && actual_count == usize::from(slot_count)),
        "out-of-range root reports exact coordinates and counts",
    );
    kani::cover(
        slot_count == 8 && symbols_count == u32::MAX,
        "original root bound",
    );
    std::mem::forget(accessors);
}

#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_symbol_oob_rejected() {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root: u16 = kani::any();
    let index: u32 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 16);
    kani::assume(symbols_count > 0);
    kani::assume(symbols_count <= 64);
    kani::assume(root < slot_count);
    kani::assume(index != u32::MAX);
    let accessors = [AccessorProgram {
        root: SlotIdx::new(root),
        path: Box::new([
            PathSegment::Index(index),
            PathSegment::Field(SymbolId::new(symbols_count)),
        ]),
    }];
    kani::assert(
        matches!(validate_accessor_paths(&accessors, slot_count, symbols_count),
            Err(ValidationError::AccessorSymbolOutOfBounds { accessor_index, segment_index, symbol, symbols_count: actual_count })
                if accessor_index == 0 && segment_index == 1 && symbol == symbols_count && actual_count == symbols_count),
        "out-of-range symbol reports exact coordinates and counts",
    );
    kani::cover(
        slot_count == 16 && symbols_count == 64,
        "original symbol bounds",
    );
    std::mem::forget(accessors);
}
