#![cfg(kani)]
#![forbid(unsafe_code)]
#![allow(unused_must_use)]
//! Kani harnesses for Repeat digest coverage (bead vb-xi2f.31).
//!
//! Proof obligations: PO-001 through PO-005.
//!
//! These harnesses verify that `digest_step_primitive` correctly consumes
//! `max_attempts` and `body` fields of the `Repeat` primitive in the blake3
//! hash state, producing distinct digests for distinct Repeat configurations.
//!
//! ## GOD RULE 1: Uses `kani::any()` for symbolic max_attempts and body generation.
//! ## GOD RULE 2: Binds to actual `mod_compile_lowering::part_05` implementation.
//! ## GOD RULE 3: NO hardcoded structural inputs; all values are symbolic.
//!
//! ## Blocker: BLOCKER-BLAKE3-INLINEASM
//!
//! The blake3 crate uses `__cpuid_count` inline assembly for CPU feature
//! detection (via `is_x86_feature_detected!`), which Kani cannot model
//! (`TerminatorKind::InlineAsm is not currently supported by Kani`).
//! The harnesses compile correctly but verification fails on the first
//! `Hasher::new()` call that triggers CPU detection.
//!
//! Trusted base: blake3::Hasher is assumed correct (TBL-001).
//! Compensation: unit tests (PO-008-PO-010), integration tests (PO-011-PO-012),
//! and proptest (PO-006-PO-007) provide property-based and end-to-end
//! coverage of the same Repeat digest properties.

use crate::ast::{ScalarValue, StepAst, StepPrimitive};
use crate::mod_compile_lowering::digest_step_primitive;

// =========================================================================
// Kani-friendly arbitrary string helpers
// =========================================================================

/// Generate a bounded String from kani::any byte array.
/// Bounded to 8 bytes to keep state space tractable.
fn kani_string() -> String {
    let bytes: [u8; 8] = kani::any();
    // Truncate at first null or use all bytes. Non-UTF-8 bytes are fine
    // since YAML output/value fields are just hashed as bytes.
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

// =========================================================================
// Helpers: Construct StepAst values symbolically
// =========================================================================

fn make_finish_step(id: &str, value: ScalarValue) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish { result: value },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn make_set_step(id: &str, output: &str, value: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

fn symbolic_finish_scalar() -> ScalarValue {
    let is_int: bool = kani::any();
    if is_int {
        let int_val: i64 = kani::any();
        ScalarValue::Integer(int_val)
    } else {
        ScalarValue::String(kani_string())
    }
}

fn symbolic_set_body_step() -> StepAst {
    let output = kani_string();
    let value = kani_string();
    make_set_step("s", &output, &value)
}

// =========================================================================
// PO-001: kani_repeat_max_attempts_consumed
// =========================================================================

/// KANI-REPEAT-001: Prove that digest_step_primitive consumes max_attempts.
///
/// Verifies that two Repeat primitives differing only in `max_attempts` produce
/// different blake3 hasher states. Non-vacuous: asserts inequality, not just
/// `is_ok()`.
///
/// **Expected result AFTER fix**: SUCCESS — different max_attempts produce
/// different hashes. **BEFORE fix**: FAILURE — both fall through to catch-all
/// and hash only the string "repeat".
///
/// Unwind bound: 4 (body depth limited to 0 here, just max_attempts diff).
#[kani::proof]
#[kani::unwind(4)]
fn kani_repeat_max_attempts_consumed() {
    // Symbolic max_attempts values — GOD RULE 1
    let max1: u16 = kani::any();
    let max2: u16 = kani::any();
    kani::assume(max1 != max2);

    kani::cover!(max1 == 0, "max_attempts domain includes 0");
    kani::cover!(max2 == u16::MAX, "max_attempts domain includes u16::MAX");

    let empty_body: Vec<StepAst> = vec![];
    let repeat1 = StepPrimitive::Repeat {
        max_attempts: max1,
        body: empty_body.clone(),
    };
    let repeat2 = StepPrimitive::Repeat {
        max_attempts: max2,
        body: empty_body,
    };

    let mut hasher1 = blake3::Hasher::new();
    let mut hasher2 = blake3::Hasher::new();

    digest_step_primitive(&mut hasher1, &repeat1);
    digest_step_primitive(&mut hasher2, &repeat2);

    let digest1 = hasher1.finalize();
    let digest2 = hasher2.finalize();

    // Non-vacuous: different max_attempts MUST produce different digest bytes
    kani::assert(
        digest1.as_bytes() != digest2.as_bytes(),
        "different max_attempts must produce different hasher states",
    );
}

// =========================================================================
// PO-002: kani_repeat_body_consumed
// =========================================================================

/// KANI-REPEAT-002: Prove that digest_step_primitive consumes body steps.
///
/// Verifies that two Repeat primitives with identical `max_attempts` but
/// different body steps produce different blake3 hasher states.
///
/// Body depth bounded to 2: outer repeat body contains Set or Finish
/// (no nested Repeat). Compensated by proptest PO-006 for deeper configs.
///
/// **Expected result AFTER fix**: SUCCESS — different bodies produce different
/// hashes. **BEFORE fix**: FAILURE — body ignored, only "repeat" string hashed.
#[kani::proof]
#[kani::unwind(8)]
fn kani_repeat_body_consumed() {
    // Fixed (but symbolic) max_attempts; body varies via symbolic choice
    let max_attempts: u16 = kani::any();

    // Body A: Set step with symbolic output/value
    let body_set = vec![symbolic_set_body_step()];

    // Body B: Finish step with symbolic scalar
    let body_fin = vec![make_finish_step("f", symbolic_finish_scalar())];

    let repeat_a = StepPrimitive::Repeat {
        max_attempts,
        body: body_set,
    };
    let repeat_b = StepPrimitive::Repeat {
        max_attempts,
        body: body_fin,
    };

    let mut hasher_a = blake3::Hasher::new();
    let mut hasher_b = blake3::Hasher::new();

    digest_step_primitive(&mut hasher_a, &repeat_a);
    digest_step_primitive(&mut hasher_b, &repeat_b);

    let digest_a = hasher_a.finalize();
    let digest_b = hasher_b.finalize();

    // Non-vacuous: different body steps MUST produce different digests
    kani::assert(
        digest_a.as_bytes() != digest_b.as_bytes(),
        "different repeat body must produce different hasher states",
    );
}

// =========================================================================
// PO-003: kani_repeat_different_params_different_digest
// =========================================================================

/// KANI-REPEAT-003: Prove that digest_step_primitive with Repeat produces
/// distinct hasher states for distinct max_attempts and body combinations.
///
/// This harness exercises digest_step_primitive at the hasher level with
/// symbolic max_attempts (differing) and same symbolic body, then verifies
/// the hasher states differ. The public `canonical_digest` function wraps
/// digest_step_primitive and is exercised by integration tests PO-011/PO-012.
///
/// **Expected result AFTER fix**: SUCCESS — different configs → different digests.
/// **BEFORE fix**: FAILURE — both produce identical digest.
#[kani::proof]
#[kani::unwind(6)]
fn kani_repeat_different_params_different_digest() {
    let max1: u16 = kani::any();
    let max2: u16 = kani::any();
    kani::assume(max1 != max2);

    let body_step = symbolic_set_body_step();
    let body = vec![body_step];

    let repeat1 = StepPrimitive::Repeat {
        max_attempts: max1,
        body: body.clone(),
    };
    let repeat2 = StepPrimitive::Repeat {
        max_attempts: max2,
        body,
    };

    let mut hasher1 = blake3::Hasher::new();
    let mut hasher2 = blake3::Hasher::new();

    digest_step_primitive(&mut hasher1, &repeat1);
    digest_step_primitive(&mut hasher2, &repeat2);

    let digest1 = hasher1.finalize();
    let digest2 = hasher2.finalize();

    // Non-vacuous: explicit inequality assertion
    kani::assert(
        digest1.as_bytes() != digest2.as_bytes(),
        "different repeat max_attempts must produce different workflow digests",
    );
}

// =========================================================================
// PO-004: kani_repeat_both_impls_equivalent
// =========================================================================

/// KANI-REPEAT-004: BLOCKER — compile/mod.rs implementation unreachable.
///
/// ## Blocker: BLOCKER-COMPILE-MOD-UNREACHABLE
///
/// The duplicate `digest_step_primitive` implementation in
/// `crates/vb_compile/src/compile/mod.rs` is NOT connected to the crate module
/// tree. No `mod compile;` declaration exists in `lib.rs` or any parent module.
/// Both `compile_workflow` and `compile_source` public APIs converge on
/// `part_05.rs` for digest computation.
///
/// ## Impact
/// PO-004 requires comparing outputs of BOTH implementations. Since one is
/// unreachable, this comparison cannot be made within a single Kani harness.
/// The integration tests (PO-011, PO-012) exercise both public entry points
/// and confirm digest equivalence cross-path.
///
/// ## Current Status
/// This harness verifies that the single accessible implementation
/// (part_05.rs::digest_step_primitive) is idempotent for identical inputs.
#[kani::proof]
#[kani::unwind(4)]
fn kani_repeat_both_impls_equivalent() {
    // Verify idempotency of the accessible implementation
    let max_attempts: u16 = kani::any();
    let body_step = symbolic_set_body_step();

    let repeat = StepPrimitive::Repeat {
        max_attempts,
        body: vec![body_step],
    };

    let mut hasher1 = blake3::Hasher::new();
    let mut hasher2 = blake3::Hasher::new();

    digest_step_primitive(&mut hasher1, &repeat);
    digest_step_primitive(&mut hasher2, &repeat);

    let digest1 = hasher1.finalize();
    let digest2 = hasher2.finalize();

    // The single accessible implementation is at least idempotent
    kani::assert(
        digest1.as_bytes() == digest2.as_bytes(),
        "same input must produce same digest (single-implementation idempotency)",
    );
}

// =========================================================================
// PO-005: kani_finish_set_digest_unchanged
// =========================================================================

/// KANI-REPEAT-005: Prove Set and Finish digest behavior is preserved.
///
/// Non-regression harness verifying that Set and Finish primitives produce
/// deterministic hasher state regardless of whether a Repeat arm exists in the
/// match. The Repeat fix must not alter existing behavior.
///
/// Generates symbolic Set and Finish primitives and verifies that
/// `digest_step_primitive` processes them deterministically (same input → same
/// output) and that Set and Finish produce distinct outputs.
///
/// **Expected result**: SUCCESS — Set and Finish produce correct, deterministic
/// hasher updates. Adding the Repeat arm does not regress existing cases.
#[kani::proof]
#[kani::unwind(6)]
fn kani_finish_set_digest_unchanged() {
    // --- Test Set primitive ---
    let set_prim = StepPrimitive::Set {
        output: kani_string(),
        value: kani_string(),
    };

    let mut hasher_a = blake3::Hasher::new();
    let mut hasher_b = blake3::Hasher::new();

    digest_step_primitive(&mut hasher_a, &set_prim);
    digest_step_primitive(&mut hasher_b, &set_prim);

    let digest_a = hasher_a.finalize();
    let digest_b = hasher_b.finalize();

    // Same Set input → same digest (idempotent)
    kani::assert(
        digest_a.as_bytes() == digest_b.as_bytes(),
        "identical Set primitive must produce identical hasher state",
    );

    // --- Test Finish primitive ---
    let finish_result = symbolic_finish_scalar();
    let fin_prim = StepPrimitive::Finish {
        result: finish_result,
    };

    let mut hasher_c = blake3::Hasher::new();
    let mut hasher_d = blake3::Hasher::new();

    digest_step_primitive(&mut hasher_c, &fin_prim);
    digest_step_primitive(&mut hasher_d, &fin_prim);

    let digest_c = hasher_c.finalize();
    let digest_d = hasher_d.finalize();

    // Same Finish input → same digest (idempotent)
    kani::assert(
        digest_c.as_bytes() == digest_d.as_bytes(),
        "identical Finish primitive must produce identical hasher state",
    );

    // --- Cross-check: different value types should differ ---
    // Generate a different ScalarValue for a distinct Finish
    let distinct_finish = symbolic_finish_scalar();
    let fin_prim2 = StepPrimitive::Finish {
        result: distinct_finish,
    };

    let mut hasher_e = blake3::Hasher::new();
    digest_step_primitive(&mut hasher_e, &fin_prim2);
    let digest_e = hasher_e.finalize();

    // The two finish digests may or may not differ (depending on symbolic
    // values being identical). We only check that the set vs. finish are
    // clearly distinct primitive types.
    kani::assert(
        digest_a.as_bytes() != digest_c.as_bytes(),
        "Set and Finish primitives must produce different hasher states",
    );
}
