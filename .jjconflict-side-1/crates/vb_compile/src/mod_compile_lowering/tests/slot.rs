use super::*;

// ─────────────────────────────────────────────────────────────────
// Test 3: slot_from_text valid integer
// Plan N6+N7 / RRO-B011
// ─────────────────────────────────────────────────────────────────
/// Verifies `slot_from_text` returns `Ok(SlotIdx)` for a valid integer string.
/// Tests a representative middle value (42).
#[test]
fn slot_from_text_valid_integer() {
    // Given: a valid integer string representation (42)
    // When: slot_from_text is called
    let result = slot_from_text("42", 0, "test.field");
    // Then: returns Ok with correct SlotIdx
    assert_eq!(
        result.expect("valid integer must parse"),
        vb_core::ids::SlotIdx::new(42),
    );
}

/// Verifies `slot_from_text` returns `Ok(SlotIdx(0))` for the minimum valid u16 boundary.
#[test]
fn slot_from_text_valid_zero_boundary() {
    // Given: text = "0"
    // When: slot_from_text is called
    let result = slot_from_text("0", 0, "test.field");
    // Then: returns Ok(SlotIdx(0))
    assert_eq!(
        result.expect("zero must parse"),
        vb_core::ids::SlotIdx::new(0),
    );
}

/// Verifies `slot_from_text` returns `Ok(SlotIdx(65535))` for the maximum valid u16 boundary.
#[test]
fn slot_from_text_valid_u16_max_boundary() {
    // Given: text = "65535" (u16::MAX)
    // When: slot_from_text is called
    let result = slot_from_text("65535", 0, "test.field");
    // Then: returns Ok(SlotIdx(65535))
    assert_eq!(
        result.expect("u16 max must parse"),
        vb_core::ids::SlotIdx::new(65535),
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 4: slot_from_text out of range (> u16::MAX)
// Plan N11 / RRO-B011
// ─────────────────────────────────────────────────────────────────
/// Verifies `slot_from_text` returns `SlotIndexOutOfRange` when the
/// numeric value exceeds `u16::MAX`.
#[test]
fn slot_from_text_out_of_range() {
    // Given: "65536" which is u16::MAX + 1
    // When: slot_from_text is called
    let result = slot_from_text("65536", 0, "test.field");
    // Then: returns SlotIndexOutOfRange with the exact value
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::SlotIndexOutOfRange { value }
                    if *value == 65536i64
                )),
                "slot_from_text must reject value exceeding u16::MAX, got: {:?}",
                errors
            );
        }
        other => panic!("expected SlotIndexOutOfRange error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 5: slot_from_text non-integer
// Plan N9 / RRO-B011
// ─────────────────────────────────────────────────────────────────
/// Verifies `slot_from_text` returns `StepFieldShape` error when the
/// text is not a valid integer.
#[test]
fn slot_from_text_non_integer() {
    // Given: a non-integer string
    // When: slot_from_text is called
    let result = slot_from_text("abc", 1, "test");
    // Then: returns StepFieldShape with "integer string" expected
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::StepFieldShape { expected, .. }
                    if *expected == "integer string"
                )),
                "slot_from_text must reject non-integer text, got: {:?}",
                errors
            );
        }
        other => panic!("expected StepFieldShape error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 6: slot_from_text empty string
// Plan N8 / RRO-B011
// ─────────────────────────────────────────────────────────────────
/// Verifies `slot_from_text` returns `StepFieldShape` error when the
/// text is an empty string.
#[test]
fn slot_from_text_empty_string() {
    // Given: an empty string
    // When: slot_from_text is called
    let result = slot_from_text("", 3, "choose.branches[].when");
    // Then: returns StepFieldShape with "non-empty primitive field" expected
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::StepFieldShape { step, field, expected }
                    if *step == 3
                    && *field == "choose.branches[].when"
                    && expected.contains("non-empty")
                )),
                "slot_from_text must reject empty string, got: {:?}",
                errors
            );
        }
        other => panic!("expected StepFieldShape error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 7: slot_from_text negative
// Plan N10 / RRO-B011
// ─────────────────────────────────────────────────────────────────
/// Verifies `slot_from_text` returns `SlotIndexOutOfRange` for a
/// negative integer, which parses successfully as i64 but fails
/// `u16::try_from`.
#[test]
fn slot_from_text_negative() {
    // Given: a negative integer string
    // When: slot_from_text is called
    let result = slot_from_text("-1", 0, "test");
    // Then: returns SlotIndexOutOfRange with value -1
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::SlotIndexOutOfRange { value }
                    if *value == -1i64
                )),
                "slot_from_text must reject negative integer, got: {:?}",
                errors
            );
        }
        other => panic!("expected SlotIndexOutOfRange error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 8: lower_choose fanout exceeds limit (defense-in-depth)
// Plan N14 / RRO-B008
// ─────────────────────────────────────────────────────────────────
/// Verifies that `lower_choose` (the second checkpoint) rejects >64
/// branches with `PrimitiveLoweringLimitExceeded`.
#[test]
fn lower_choose_fanout_exceeds_limit() {
    // Given: 65 SlotBranch entries
    let branches: Vec<vb_core::SlotBranch> = (0..65u16)
        .map(|i| vb_core::SlotBranch {
            condition: vb_core::ids::SlotIdx::new(i),
            target: vb_core::ids::StepIdx::new(100),
        })
        .collect();
    let mut builder = crate::SlotCompiler::new();
    // When: lower_choose is called with 65 branches
    let result = lower_choose(vb_core::ids::StepIdx::new(0), branches, None, &mut builder);
    // Then: returns PrimitiveLoweringLimitExceeded
    assert!(
        matches!(
            result,
            Err(crate::CompileError::PrimitiveLoweringLimitExceeded {
                primitive: "choose",
                field: "branches",
                value: 65,
                limit: 64,
            })
        ),
        "lower_choose must reject 65 branches at defense-in-depth, got: {:?}",
        result
    );
}
