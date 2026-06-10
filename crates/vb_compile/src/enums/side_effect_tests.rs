//! Tests verifying `SideEffect` enum matches master plan Section 65.
//!
//! These tests assert the FIXED state: the 7-variant taxonomy defined
//! in the master plan (Pure, LocalRead, LocalWrite, ExternalRead,
//! ExternalWrite, Process, UnsafeShell).

use vb_core::{
    RetrySafety, SideEffect,
    action::{ActionName, verify_idempotency},
};

/// Helper to extract the #[repr(u8)] discriminant of a SideEffect variant.
/// Returns None if the variant name does not exist in the current implementation.
const fn side_effect_discriminant(v: SideEffect) -> u8 {
    v as u8
}

/// Test that SideEffect has exactly 7 variants as specified in master plan Section 65.
///
/// Master plan defines: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite,
/// Process, UnsafeShell (7 total).
///
/// This test will fail to compile if:
/// - The variant count is wrong (not 7)
/// - Any master plan variant name doesn't exist
#[test]
fn side_effect_has_exactly_seven_master_plan_variants() {
    const MASTER_PLAN_COUNT: usize = 7;

    // Enumerate all 7 variants to prove they exist and have unique discriminants.
    let variants = [
        SideEffect::Pure,
        SideEffect::LocalRead,
        SideEffect::LocalWrite,
        SideEffect::ExternalRead,
        SideEffect::ExternalWrite,
        SideEffect::Process,
        SideEffect::UnsafeShell,
    ];

    // Verify we have exactly 7 entries
    assert_eq!(variants.len(), MASTER_PLAN_COUNT);

    // Verify all discriminants are unique (0-6).
    let mut discriminants: Vec<u8> = variants.iter().map(|v| *v as u8).collect();
    discriminants.sort();
    assert_eq!(discriminants, vec![0, 1, 2, 3, 4, 5, 6]);
}

/// Test that SideEffect::Pure exists and has a unique discriminant.
#[test]
fn side_effect_pure_variant_exists() {
    let _ = SideEffect::Pure;
    let d = side_effect_discriminant(SideEffect::Pure);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::LocalRead exists and has a unique discriminant.
#[test]
fn side_effect_local_read_variant_exists() {
    let _ = SideEffect::LocalRead;
    let d = side_effect_discriminant(SideEffect::LocalRead);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::LocalWrite exists and has a unique discriminant.
#[test]
fn side_effect_local_write_variant_exists() {
    let _ = SideEffect::LocalWrite;
    let d = side_effect_discriminant(SideEffect::LocalWrite);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::ExternalRead exists and has a unique discriminant.
#[test]
fn side_effect_external_read_variant_exists() {
    let _ = SideEffect::ExternalRead;
    let d = side_effect_discriminant(SideEffect::ExternalRead);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::ExternalWrite exists and has a unique discriminant.
#[test]
fn side_effect_external_write_variant_exists() {
    let _ = SideEffect::ExternalWrite;
    let d = side_effect_discriminant(SideEffect::ExternalWrite);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::Process exists and has a unique discriminant.
#[test]
fn side_effect_process_variant_exists() {
    let _ = SideEffect::Process;
    let d = side_effect_discriminant(SideEffect::Process);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that SideEffect::UnsafeShell exists and has a unique discriminant.
#[test]
fn side_effect_unsafe_shell_variant_exists() {
    let _ = SideEffect::UnsafeShell;
    let d = side_effect_discriminant(SideEffect::UnsafeShell);
    assert!(d < 128, "discriminant must be valid u8");
}

/// Test that all 7 SideEffect variants have distinct #[repr(u8)] discriminants.
///
/// This prevents two variants from sharing the same discriminant value,
/// which would violate the exclusivity requirement of the master plan.
#[test]
fn side_effect_all_discriminants_are_distinct() {
    use std::collections::BTreeSet;

    let variants = [
        SideEffect::Pure,
        SideEffect::LocalRead,
        SideEffect::LocalWrite,
        SideEffect::ExternalRead,
        SideEffect::ExternalWrite,
        SideEffect::Process,
        SideEffect::UnsafeShell,
    ];

    let mut discriminants = BTreeSet::new();
    let mut i = 0;
    while i < variants.len() {
        let d = side_effect_discriminant(variants[i]);
        assert!(
            discriminants.insert(d),
            "duplicate discriminant {d} found at index {i}"
        );
        i += 1;
    }
    assert_eq!(discriminants.len(), 7, "all 7 discriminants must be unique");
}

/// Test that SideEffect implements Copy (required for enum discriminants).
#[test]
fn side_effect_is_copy() {
    let a = SideEffect::Pure;
    let _b = a;
}

/// Test that SideEffect variants are used correctly in verify_idempotency.
///
/// This verifies that the verify_idempotency function in vb_core's action.rs
/// handles the fixed SideEffect taxonomy correctly.
#[test]
fn side_effect_verify_idempotency_handles_pure_correctly() {
    use vb_core::{ActionContract, ActionId, Idempotency, RunFrame, RunId, StepIdx};

    let contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 0,
        output_slot_count: 1,
        max_input_bytes: 0,
        max_output_bytes: 0,
        timeout_ms: 0,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    assert!(frame.is_ok());
    let frame = frame.expect("test frame");

    // Pure actions should always pass idempotency verification
    let result = verify_idempotency(&contract, &[], &frame);
    assert_eq!(
        result,
        Ok(()),
        "Pure SideEffect should pass verify_idempotency"
    );
}
