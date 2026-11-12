use super::*;

// ─────────────────────────────────────────────────────────────────
// vb-xi2f.13 new tests: choose_width overflow, slot_from_text,
// fanout defense-in-depth, anti-hallucination, slot allocation,
// condition/body slot disjointness, otherwise span
// ─────────────────────────────────────────────────────────────────
// ─────────────────────────────────────────────────────────────────
// Test 1: choose_width includes recursive primitive width
// Plan N2 / RRO-B004
// ─────────────────────────────────────────────────────────────────
#[test]
fn choose_width_includes_recursive_primitive_width() {
    let nested_step = crate::StepAst {
        id: "unsupported".to_string(),
        name: None,
        condition: None,
        primitive: crate::StepPrimitive::ForEach {
            variable: "x".to_string(),
            input: "items".to_string(),
            at_once: None,
            body: vec![],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let branches = vec![crate::ChooseBranch {
        when: "0".to_string(),
        steps: vec![nested_step],
    }];
    let result = choose_width(&branches);
    assert_eq!(result, Ok(8));
}

// ─────────────────────────────────────────────────────────────────
// Test 2: body_width overflow via checked_add
// Plan N3 / RRO-B004
// ─────────────────────────────────────────────────────────────────
/// Verifies that `body_width` returns `StepIndexOutOfRange` when the
/// `checked_add` accumulator overflows.
///
/// Uses `overhead = usize::MAX` with one body step (adds 1) to trigger
/// the overflow on the first iteration.
#[test]
fn choose_width_body_step_overflow_returns_error() {
    // Given: a body with one Set step and overhead = usize::MAX
    let body = vec![choose_body_set_step("overflow_step", "1")];
    // When: body_width is called with overflow-inducing overhead
    let result = body_width(&body, usize::MAX);
    // Then: returns StepIndexOutOfRange from checked_add failure
    assert!(
        matches!(
            result,
            Err(crate::CompileError::StepIndexOutOfRange { value }) if value == usize::MAX
        ),
        "body_width must return StepIndexOutOfRange on checked_add overflow, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test: add_body_offset overflow (checked_add at part_14.rs:140)
// Plan N5 / RRO-B005
// ─────────────────────────────────────────────────────────────────
/// Verifies that `add_body_offset` returns `StepIndexOutOfRange` when
/// `checked_add` between `start_offset` and `index` overflows `u16`.
/// Directly tests the overflow guard at part_14.rs:140.
#[test]
fn add_body_offset_overflow_returns_error() {
    use crate::mod_compile_lowering::part_14::add_body_offset;
    // Given: start_offset = u16::MAX, index = 1 (sum exceeds u16::MAX)
    // When: add_body_offset is called
    let result = add_body_offset(u16::MAX, 1, 0);
    // Then: returns StepIndexOutOfRange
    assert!(
        matches!(
            result,
            Err(ref errors) if errors.0.iter().any(|e| matches!(
                e,
                crate::CompileError::StepIndexOutOfRange { .. }
            ))
        ),
        "add_body_offset must return StepIndexOutOfRange on overflow, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test: cursor overflow during body lowering (checked_add at part_14.rs:34,53)
// Plan N4 / RRO-B005
// ─────────────────────────────────────────────────────────────────
/// Verifies that the `cursor.checked_add(width)` overflow guards inside
/// `lower_canonical_choose` propagate the correct error when body step
/// offsets overflow `u16`. Uses `emit_choose_branch_body` with a
/// `start_offset` of `u16::MAX` and two body steps to trigger
/// `add_body_offset` overflow, which exercises the same `checked_add`
/// pattern as the cursor accumulation at part_14.rs:34 and part_14.rs:53.
///
/// Direct cursor overflow requires >65535 body steps total (impractical
/// to construct); this test covers the same error variant and guard
/// mechanism through the reachable `emit_choose_branch_body` path.
#[test]
fn lower_choose_body_cursor_overflow_returns_error() {
    use crate::mod_compile_lowering::part_14::emit_choose_branch_body;
    // Given: start_offset = u16::MAX, body with 2 Set steps
    //   i=0: add_body_offset(u16::MAX, 0, _) → Ok(u16::MAX)
    //   i=1: add_body_offset(u16::MAX, 1, _) → overflow → StepIndexOutOfRange
    let body = vec![
        choose_body_set_step("cursor_a", "1"),
        choose_body_set_step("cursor_b", "2"),
    ];
    let mut builder = crate::SlotCompiler::new();
    // When: emit_choose_branch_body is called with overflow-inducing start_offset
    let result = emit_choose_branch_body(
        &body,
        vb_core::ids::StepIdx::new(0),
        u16::MAX,
        0,
        vb_core::ids::StepIdx::new(100),
        &mut builder,
    );
    // Then: returns StepIndexOutOfRange from add_body_offset overflow
    assert!(
        matches!(
            result,
            Err(ref errors) if errors.0.iter().any(|e| matches!(
                e,
                crate::CompileError::StepIndexOutOfRange { .. }
            ))
        ),
        "emit_choose_branch_body must return StepIndexOutOfRange on cursor overflow, got: {:?}",
        result
    );
}
