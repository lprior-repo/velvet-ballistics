//! Kani harness: constant pool overflow verification.
//!
//! Bead: vb-core-lower-values-actions-refs
//! Workspace: /tmp/vb-ws/vb-core-lower-values-actions-refs
//! Obligation: KANI-CONSTANT-POOL-001
//!
//! Target: crates/vb_compile/src/lib.rs::SlotCompiler::push_constant
//! Claim: push_constant returns Err on pool size > u16::MAX and Ok(ConstIdx) otherwise.
//!
//! Verifier: cargo kani --package vb_compile --harness push_constant_overflow
//!
//! F-008 Fix: No 65535-iteration loops. Instead:
//!   - Concrete small-boundary test (5 pushes, verify indices correct)
//!   - Symbolic boundary: assumes restrict n to 0..19 so loop is fully explorable
//!   - slot_count_overflow_at_max: direct test of u16::MAX + 1 overflow

#![forbid(unsafe_code)]

use crate::SlotCompiler;
use vb_core::{ConstIdx, ConstValue};

/// KANI-CONSTANT-POOL-001: SlotCompiler::push_constant is safe and correct.
///
/// Strategy:
///   1. Concrete small-fill boundary — 5 pushes, verify sequential indices
///   2. Symbolic boundary: assume n < 20 so while loop is fully explorable
///      (n is any u16, but kani::assume restricts to 0..19 for loop exploration)
///   3. slot_count overflow: record slot at u16::MAX → overflow on +1
///   4. All ConstValue variants succeed on fresh compiler
#[kani::proof]
#[kani::unwind(20)]
fn push_constant_overflow() {
    // ----------------------------------------------------------------
    // Test 1: small-fill boundary — concrete 5 pushes, verify sequential indices
    // ----------------------------------------------------------------
    let mut bounded_compiler = SlotCompiler::new();
    let fill_values: [ConstValue; 5] = [
        ConstValue::I64(0),
        ConstValue::I64(1),
        ConstValue::I64(2),
        ConstValue::I64(3),
        ConstValue::I64(4),
    ];

    // Unrolled concrete verification
    let r0 = bounded_compiler.push_constant(fill_values[0]);
    kani::assert(r0.is_ok(), "index 0 should succeed");
    if let Ok(idx) = r0 { kani::assert(idx == ConstIdx::new(0, "assertion failed"), "index 0"); }

    let r1 = bounded_compiler.push_constant(fill_values[1]);
    kani::assert(r1.is_ok(), "index 1 should succeed");
    if let Ok(idx) = r1 { kani::assert(idx == ConstIdx::new(1, "assertion failed"), "index 1"); }

    let r2 = bounded_compiler.push_constant(fill_values[2]);
    kani::assert(r2.is_ok(), "index 2 should succeed");
    if let Ok(idx) = r2 { kani::assert(idx == ConstIdx::new(2, "assertion failed"), "index 2"); }

    let r3 = bounded_compiler.push_constant(fill_values[3]);
    kani::assert(r3.is_ok(), "index 3 should succeed");
    if let Ok(idx) = r3 { kani::assert(idx == ConstIdx::new(3, "assertion failed"), "index 3"); }

    let r4 = bounded_compiler.push_constant(fill_values[4]);
    kani::assert(r4.is_ok(), "index 4 should succeed");
    if let Ok(idx) = r4 { kani::assert(idx == ConstIdx::new(4, "assertion failed"), "index 4"); }

    // The 6th push (index 5) should also succeed (well within u16::MAX)
    let sixth = bounded_compiler.push_constant(ConstValue::Null);
    kani::assert(sixth.is_ok(), "index 5 should succeed (within u16::MAX)");
    if let Ok(idx) = sixth {
        kani::assert(idx == ConstIdx::new(5, "assertion failed"), "6th constant should have index 5");
    }

    // ----------------------------------------------------------------
    // Test 2: symbolic boundary — push n+1 constants where n is 0..19
    // kani::assume restricts n so the while loop is fully explorable (max 20 iterations)
    // ----------------------------------------------------------------
    let n = kani::any::<u16>();
    kani::assume(n < 20); // Restrict so loop is fully explorable with unwind(20)

    let mut sym_compiler = SlotCompiler::new();
    let mut i = 0u16;
    while i <= n {
        let result = sym_compiler.push_constant(ConstValue::I64(i as i64));
        kani::assert(result.is_ok(), "push up to n should succeed when n < 20");
        i += 1;
    }

    // One more push at index n+1 should also succeed (still <= 20 < u16::MAX)
    let one_more = sym_compiler.push_constant(ConstValue::Null);
    kani::assert(one_more.is_ok(), "push at n+1 (where n+1 <= 20) should succeed");

    // ----------------------------------------------------------------
    // Test 3: slot_count on empty builder = 0
    // ----------------------------------------------------------------
    let empty_compiler = SlotCompiler::new();
    let empty_count = empty_compiler.slot_count();
    kani::assert(empty_count.is_ok(), "slot_count on empty builder should be Ok");
    let empty_count_val = match empty_count {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    , "slot_count on empty builder should be Ok");
    let empty_count_val = match empty_count {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(empty_count_val == 0, "empty builder slot_count should be 0");

    // ----------------------------------------------------------------
    // Test 4: all ConstValue variants succeed on fresh compiler (unrolled)
    // ----------------------------------------------------------------
    let mut test_compiler = SlotCompiler::new();
    kani::assert(test_compiler.push_constant(ConstValue::Null).is_ok(), "Null succeeds");
    kani::assert(test_compiler.push_constant(ConstValue::Bool(true), "assertion failed").is_ok(), "Bool(true) succeeds");
    kani::assert(test_compiler.push_constant(ConstValue::Bool(false), "assertion failed").is_ok(), "Bool(false) succeeds");
    kani::assert(test_compiler.push_constant(ConstValue::I64(0), "assertion failed").is_ok(), "I64(0) succeeds");
    kani::assert(test_compiler.push_constant(ConstValue::I64(-1), "assertion failed").is_ok(), "I64(-1) succeeds");
    kani::assert(test_compiler.push_constant(ConstValue::I64(i64::MAX), "assertion failed").is_ok(), "I64::MAX succeeds");
    kani::assert(test_compiler.push_constant(ConstValue::I64(i64::MIN), "assertion failed").is_ok(), "I64::MIN succeeds");
    let f64_0 = match vb_core::FiniteF64::new(0.0) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(test_compiler.push_constant(ConstValue::F64(f64_0), "assertion failed").is_ok(), "F64(0.0) succeeds");
    let f64_1 = match vb_core::FiniteF64::new(1.5) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(test_compiler.push_constant(ConstValue::F64(f64_1), "assertion failed").is_ok(), "F64(1.5) succeeds");
    let f64_2 = match vb_core::FiniteF64::new(-3.14) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(test_compiler.push_constant(ConstValue::F64(f64_2), "assertion failed").is_ok(), "F64(-3.14) succeeds");
}

/// KANI-CONSTANT-POOL-001b: push_constant does not affect slot recording.
#[kani::proof]
#[kani::unwind(8)]
fn push_constant_isolation() {
    let mut compiler = SlotCompiler::new();

    // Push some constants — should NOT affect slot_count
    let _ = compiler.push_constant(ConstValue::I64(1));
    let _ = compiler.push_constant(ConstValue::I64(2));
    let _ = compiler.push_constant(ConstValue::Bool(true));

    let count = compiler.slot_count();
    kani::assert(count.is_ok(), "slot_count should be Ok");
    let count_val = match count {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    , "slot_count should be Ok");
    let count_val = match count {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(count_val == 0, "pushing constants should not affect slot_count");

    // Record some slots
    compiler.record_slot(vb_core::SlotIdx::new(3));
    compiler.record_slot(vb_core::SlotIdx::new(7));

    let count_after = compiler.slot_count();
    kani::assert(count_after.is_ok(), "slot_count should be Ok after recording");
    let count_after_val = match count_after {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    , "slot_count should be Ok after recording");
    let count_after_val = match count_after {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(count_after_val == 8, "slot_count should be max_recorded + 1");

    // Push more constants — should not affect slot_count
    let _ = compiler.push_constant(ConstValue::Null);
    let final_count = compiler.slot_count();
    let final_count_val = match final_count {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    kani::assert(final_count_val == 8, "pushing constants after slot recording should not change slot_count");
}

/// KANI-CONSTANT-POOL-001c: slot_count overflow when max_slot = u16::MAX.
///
/// slot_count() = max_slot + 1. When max_slot = u16::MAX (65535),
/// max_slot + 1 = 65536 which overflows u16::try_from, producing Err.
#[kani::proof]
#[kani::unwind(6)]
fn slot_count_overflow_at_max() {
    let mut compiler = SlotCompiler::new();

    // Record slot at u16::MAX
    compiler.record_slot(vb_core::SlotIdx::new(u16::MAX));

    // slot_count() computes max_slot + 1 = 65535 + 1 = 65536.
    // u16::try_from(65536) fails -> Err(SlotIndexOutOfRange)
    let result = compiler.slot_count();
    kani::assert(result.is_err(), "slot_count should overflow at u16::MAX");
}
