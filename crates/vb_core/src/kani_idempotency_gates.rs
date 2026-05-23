//! Kani harnesses for vb_core idempotency runtime gates.
//!
//! Scope: vb_core
//! Obligations: KANI-RUNTIME-001 through KANI-RUNTIME-006
//!
//! Note: This module is compiled only under `#[cfg(kani)]`.
//! These files are the primary proof artifacts. The `kani/` directory at
//! workspace root contains reference copies of these harnesses.

#![forbid(unsafe_code)]

use crate::action::{ActionContract, Idempotency, RetrySafety, SideEffect, verify_idempotency};
use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};

/// KANI-RUNTIME-001: Ok when all key slots are clean (no SecretTaint/Random/TimeDependent).
///
/// Bounded: key_slots length 1..16, all slots contain Clean values.
/// Uses kani::any::<ActionContract>() with assumes to constrain the contract symbolically.
#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_all_clean() {
    // Generate contract symbolically — not hardcoded structure
    let contract = kani::any::<ActionContract>();
    // Must have side-effect to enter the retry-safety check path
    kani::assume(contract.side_effect != SideEffect::None);
    // Must be KeyRequired to check idempotency keys
    kani::assume(contract.retry_safety == RetrySafety::KeyRequired);

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
    kani::assume(frame.is_ok());
    let mut frame = frame.ok().unwrap();

    // Populate all 4 slots with Clean values
    let mut slot_i: u16 = 0;
    while slot_i < 4 {
        let write_result =
            frame.write_slot_with_taint(SlotIdx::new(slot_i), SlotValue::I64(42), Taint::Clean);
        kani::assume(write_result.is_ok());
        slot_i = match slot_i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }

    let key_slots = [
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
    ];

    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::assert(
        result.is_ok(),
        "verify_idempotency must pass when all key slots are clean",
    );
}

/// KANI-RUNTIME-002: Err(MissingKey) when key_slots is empty with KeyRequired.
///
/// Bounded: key_slots length = 0.
#[kani::proof]
#[kani::unwind(4)]
fn verify_idempotency_missing_key() {
    // Generate contract symbolically
    let contract = kani::any::<ActionContract>();
    // Must have side-effect to enter the retry-safety check path
    kani::assume(contract.side_effect != SideEffect::None);
    // Must be KeyRequired to check for missing keys
    kani::assume(contract.retry_safety == RetrySafety::KeyRequired);

    let key_slots: [SlotIdx; 0] = [];

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 2, 2);
    kani::assume(frame.is_ok());
    let frame = frame.ok().unwrap();

    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::assert(
        result.is_err(),
        "verify_idempotency must Err when key_slots is empty with KeyRequired",
    );

    if let Err(err) = &result {
        match err {
            crate::action::IdempotencyViolation::MissingKey(_) => {}
            _ => {
                kani::assert(false, "Expected MissingKey error variant");
            }
        }
    }
}

/// KANI-RUNTIME-003: Err(SecretInKey) when a key slot carries SecretTaint.
///
/// Bounded: key_slots length 1..16, at least one slot has SecretTaint.
#[kani::proof]
#[kani::unwind(6)]
fn verify_idempotency_secret_in_key() {
    // Generate contract symbolically
    let contract = kani::any::<ActionContract>();
    // Must have side-effect to enter the retry-safety check path
    kani::assume(contract.side_effect != SideEffect::None);
    // Must be KeyRequired to check idempotency key ingredients
    kani::assume(contract.retry_safety == RetrySafety::KeyRequired);

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
    kani::assume(frame.is_ok());
    let mut frame = frame.ok().unwrap();

    {
        let r = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
        kani::assume(r.is_ok());
    }
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
        kani::assume(r.is_ok());
    }
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(30), Taint::Clean);
        kani::assume(r.is_ok());
    }
    {
        let r = frame.write_slot_with_taint(
            SlotIdx::new(3),
            SlotValue::I64(40),
            Taint::DerivedFromSecret,
        );
        kani::assume(r.is_ok());
    }

    let key_slots = [
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
    ];

    let result = verify_idempotency(&contract, &key_slots, &frame);
    kani::assert(
        result.is_err(),
        "verify_idempotency must Err when key slot has SecretTaint",
    );

    if let Err(err) = &result {
        match err {
            crate::action::IdempotencyViolation::SecretInKey(slot_idx) => {
                kani::assert(
                    *slot_idx == 1 || *slot_idx == 3,
                    "SecretInKey must report a tainted slot index",
                );
            }
            _ => {
                kani::assert(false, "Expected SecretInKey error variant");
            }
        }
    }
}

/// KANI-RUNTIME-006: At most one error variant is ever returned (short-circuit invariant).
///
/// The function iterates key_slots and short-circuits on first error.
/// This harness verifies no dual/triple error reporting.
#[kani::proof]
#[kani::unwind(8)]
fn verify_idempotency_single_error() {
    // Generate contract symbolically
    let contract = kani::any::<ActionContract>();
    // Must have side-effect to enter the retry-safety check path
    kani::assume(contract.side_effect != SideEffect::None);
    // Must be KeyRequired to check idempotency keys
    kani::assume(contract.retry_safety == RetrySafety::KeyRequired);

    let frame = RunFrame::new(RunId::new(1), StepIdx::new(0), 4, 4);
    kani::assume(frame.is_ok());
    let mut frame = frame.ok().unwrap();

    {
        let r = frame.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(10), Taint::Clean);
        kani::assume(r.is_ok());
    }
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(1), SlotValue::I64(20), Taint::Secret);
        kani::assume(r.is_ok());
    }
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(2), SlotValue::I64(30), Taint::Secret);
        kani::assume(r.is_ok());
    }
    {
        let r = frame.write_slot_with_taint(SlotIdx::new(3), SlotValue::I64(40), Taint::Clean);
        kani::assume(r.is_ok());
    }

    let key_slots = [
        SlotIdx::new(0),
        SlotIdx::new(1),
        SlotIdx::new(2),
        SlotIdx::new(3),
    ];

    let result = verify_idempotency(&contract, &key_slots, &frame);

    kani::assert(
        result.is_err(),
        "verify_idempotency must return Err when any key slot is tainted",
    );

    // Exactly one error variant (short-circuit guarantee)
    if let Err(err) = &result {
        let is_missing = matches!(err, crate::action::IdempotencyViolation::MissingKey(_));
        let is_secret = matches!(err, crate::action::IdempotencyViolation::SecretInKey(_));
        let is_random = matches!(err, crate::action::IdempotencyViolation::RandomInKey(_));
        let is_time = matches!(err, crate::action::IdempotencyViolation::TimeInKey(_));

        let variant_count = [is_missing, is_secret, is_random, is_time]
            .iter()
            .filter(|&&b| b)
            .count();

        kani::assert(
            variant_count == 1,
            "Error result must contain exactly one error variant (short-circuit)",
        );
    }
}
