//! Unit tests for `digest_step_primitive` Wait arm and `canonical_primitive_name`.
//!
//! These tests exercise the exact function being fixed for vb-xi2f.32:
//! direct calls to `digest_step_primitive` with `StepPrimitive::Wait` variants.
//!
//! See `.beads/vb-xi2f.32/test-plan.md` Sections 9.1 and 9.2.

use crate::mod_compile_lowering::{canonical_primitive_name, digest_step_primitive};
use blake3::Hasher;
use proptest::prelude::*;
use vb_yaml::ast::{ScalarValue, StepPrimitive};

// ---------------------------------------------------------------------------
// Section 9.1: Direct unit tests for `digest_step_primitive` Wait arm
// ---------------------------------------------------------------------------

/// Verify that the explicit Wait arm includes more than just the primitive
/// name in the hasher state. The pre-fix catch-all path only hashed `b"wait"`;
/// the fix adds event and timeout field bytes.
#[test]
fn digest_step_wait_includes_wait_label_when_wait_primitive_is_hashed() {
    // Given: a Wait primitive with both event and timeout present
    let wait = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("30".into()),
    };

    // When: digest_step_primitive processes the Wait primitive
    let mut hasher_with_wait = Hasher::new();
    digest_step_primitive(&mut hasher_with_wait, &wait);
    let digest_wait = hasher_with_wait.finalize();

    // Simulate the pre-fix catch-all: only hash the primitive name "wait"
    let mut hasher_catch_all = Hasher::new();
    hasher_catch_all.update(b"wait");
    let digest_catch_all = hasher_catch_all.finalize();

    // Then: the explicit Wait arm must include field bytes, not just the name
    assert_ne!(
        digest_wait, digest_catch_all,
        "Wait arm must hash field bytes beyond just the primitive name 'wait'; \
         pre-fix catch-all hash checked only the name"
    );
}

/// Verify that different event values produce different hasher states
/// when the timeout field is held constant. This proves C1 field sensitivity.
#[test]
fn event_field_affects_hasher_state_when_event_values_differ() {
    // Given: two Wait primitives with different event values, same timeout
    let wait_a = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("30".into()),
    };
    let wait_b = StepPrimitive::Wait {
        event: Some("1".into()),
        timeout: Some("30".into()),
    };

    // When: both are hashed independently
    let mut hasher_a = Hasher::new();
    digest_step_primitive(&mut hasher_a, &wait_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = Hasher::new();
    digest_step_primitive(&mut hasher_b, &wait_b);
    let digest_b = hasher_b.finalize();

    // Then: different event values must produce different digests
    assert_ne!(
        digest_a, digest_b,
        "Wait primitives with different event values must produce different hasher states"
    );
}

/// Verify that different timeout values produce different hasher states
/// when the event field is held constant. This proves C1 field sensitivity.
#[test]
fn timeout_field_affects_hasher_state_when_timeout_values_differ() {
    // Given: two Wait primitives with same event, different timeout values
    let wait_a = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("10".into()),
    };
    let wait_b = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("20".into()),
    };

    // When: both are hashed independently
    let mut hasher_a = Hasher::new();
    digest_step_primitive(&mut hasher_a, &wait_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = Hasher::new();
    digest_step_primitive(&mut hasher_b, &wait_b);
    let digest_b = hasher_b.finalize();

    // Then: different timeout values must produce different digests
    assert_ne!(
        digest_a, digest_b,
        "Wait primitives with different timeout values must produce different hasher states"
    );
}

/// Verify that the sentinel `b"none"` for absent event is unambiguous:
/// `event=None` (sentinel) must not collide with `event=Some("none_sentinel_probe")`.
/// This proves the C3 sentinel contract.
#[test]
fn none_event_uses_none_sentinel_when_event_is_absent() {
    // Given: WaitUntil (event=None) and WaitEvent with event text "none_sentinel_probe"
    let wait_until = StepPrimitive::Wait {
        event: None,
        timeout: Some("30".into()),
    };
    let wait_event_probe = StepPrimitive::Wait {
        event: Some("none_sentinel_probe".into()),
        timeout: Some("30".into()),
    };

    // When: both are hashed independently
    let mut hasher_until = Hasher::new();
    digest_step_primitive(&mut hasher_until, &wait_until);
    let digest_until = hasher_until.finalize();

    let mut hasher_probe = Hasher::new();
    digest_step_primitive(&mut hasher_probe, &wait_event_probe);
    let digest_probe = hasher_probe.finalize();

    // Then: the sentinel must not collide with an explicit field value
    assert_ne!(
        digest_until, digest_probe,
        "event=None must produce a different digest than \
         event=Some(\"none_sentinel_probe\"); sentinel b\"none\" must be unambiguous"
    );
}

/// Verify that the sentinel `b"none"` for absent timeout is unambiguous:
/// `timeout=None` (sentinel) must not collide with `timeout=Some("none_sentinel_probe")`.
/// This proves the C3 sentinel contract.
#[test]
fn none_timeout_uses_none_sentinel_when_timeout_is_absent() {
    // Given: WaitEvent unbounded (timeout=None) and WaitEvent with timeout text "none_sentinel_probe"
    let wait_unbounded = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: None,
    };
    let wait_probe = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("none_sentinel_probe".into()),
    };

    // When: both are hashed independently
    let mut hasher_unbounded = Hasher::new();
    digest_step_primitive(&mut hasher_unbounded, &wait_unbounded);
    let digest_unbounded = hasher_unbounded.finalize();

    let mut hasher_probe = Hasher::new();
    digest_step_primitive(&mut hasher_probe, &wait_probe);
    let digest_probe = hasher_probe.finalize();

    // Then: the sentinel must not collide with an explicit field value
    assert_ne!(
        digest_unbounded, digest_probe,
        "timeout=None must produce a different digest than \
         timeout=Some(\"none_sentinel_probe\"); sentinel b\"none\" must be unambiguous"
    );
}

/// Verify that `digest_step_primitive` is deterministic: calling it twice
/// with the same Wait primitive on two fresh hashers produces identical
/// `finalize()` outputs.
#[test]
fn digest_step_wait_arm_is_deterministic_when_same_input_hashed_twice() {
    // Given: a Wait primitive
    let wait = StepPrimitive::Wait {
        event: Some("42".into()),
        timeout: Some("99".into()),
    };

    // When: hashed twice on independent hashers
    let mut hasher1 = Hasher::new();
    digest_step_primitive(&mut hasher1, &wait);
    let digest1 = hasher1.finalize();

    let mut hasher2 = Hasher::new();
    digest_step_primitive(&mut hasher2, &wait);
    let digest2 = hasher2.finalize();

    // Then: both digests must be identical
    assert_eq!(
        digest1, digest2,
        "Same Wait primitive hashed twice must produce identical digests"
    );
}

/// Verify that the explicit Wait arm never collides with the pre-fix
/// catch-all behavior. The catch-all only hashed `b"wait"`; the explicit
/// arm additionally hashes event and timeout field bytes. These must
/// produce different final hashes.
#[test]
fn digest_step_wait_vs_catch_all_never_collides_when_explicit_arm_is_active() {
    // Given: a Wait primitive with known field values
    let wait = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("5".into()),
    };

    // When: explicit Wait arm processes the primitive
    let mut hasher_explicit = Hasher::new();
    digest_step_primitive(&mut hasher_explicit, &wait);
    let digest_explicit = hasher_explicit.finalize();

    // Simulate the pre-fix catch-all path: only hash "wait" (no field bytes)
    let mut hasher_catch_all = Hasher::new();
    hasher_catch_all.update(b"wait");
    let digest_catch_all = hasher_catch_all.finalize();

    // Then: explicit arm and catch-all must never collide
    assert_ne!(
        digest_explicit, digest_catch_all,
        "Explicit Wait arm must produce a different hash than the pre-fix catch-all \
         which only hashed the primitive name b\"wait\""
    );
}

/// Verify that `digest_step_primitive` does not panic for any of the three
/// legal Wait shapes: WaitUntil (event=None, timeout=Some), WaitEvent
/// unbounded (event=Some, timeout=None), and WaitEvent bounded (event=Some,
/// timeout=Some). This is the direct unit complement to Kani KH-1.
#[test]
fn digest_step_wait_no_panic_for_three_legal_shapes_when_any_wait_configuration_used() {
    let shapes: [StepPrimitive; 3] = [
        // WaitUntil: event absent, timeout present
        StepPrimitive::Wait {
            event: None,
            timeout: Some("5".into()),
        },
        // WaitEvent unbounded: event present, timeout absent
        StepPrimitive::Wait {
            event: Some("0".into()),
            timeout: None,
        },
        // WaitEvent bounded: both present
        StepPrimitive::Wait {
            event: Some("0".into()),
            timeout: Some("30".into()),
        },
    ];

    for shape in &shapes {
        let mut hasher = Hasher::new();
        digest_step_primitive(&mut hasher, shape);
        // The assertion is that we reach this point without panicking.
        // We also verify the hasher produced output (not a zero state).
        let digest = hasher.finalize();
        assert!(
            !digest.as_bytes().iter().all(|b| *b == 0),
            "digest_step_primitive must produce non-zero hash output for legal Wait shape"
        );
    }
}

/// Verify that one Wait shape (WaitUntil) produces the expected digest
/// pattern: `b"wait"` + sentinel event + timeout value. We test this by
/// constructing WaitUntil and comparing against WaitUntil with different
/// timeout — they must differ, proving the timeout is included.
#[test]
fn wait_until_hashes_label_sentinel_and_timeout_when_event_is_absent() {
    // Given: two WaitUntil configurations with different timeouts
    let until_a = StepPrimitive::Wait {
        event: None,
        timeout: Some("5".into()),
    };
    let until_b = StepPrimitive::Wait {
        event: None,
        timeout: Some("10".into()),
    };

    // When: both are hashed
    let mut hasher_a = Hasher::new();
    digest_step_primitive(&mut hasher_a, &until_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = Hasher::new();
    digest_step_primitive(&mut hasher_b, &until_b);
    let digest_b = hasher_b.finalize();

    // Then: different timeouts produce different digests (B2 contract)
    assert_ne!(
        digest_a, digest_b,
        "WaitUntil with different timeouts must produce different digests; \
         timeout field must be included in hasher state"
    );
}

/// Verify C2 discriminator at the `digest_step_primitive` unit level:
/// WaitUntil (event=None, timeout=Some) must produce a different hash than
/// WaitEvent (event=Some, timeout=None) due to positional `b"none"` sentinel
/// discrimination (DD-4 refinement). The `b"none"` sentinel in the event
/// position for WaitUntil vs actual event text for WaitEvent guarantees
/// distinct hasher states.
#[test]
fn digest_step_primitive_discriminates_wait_until_from_wait_event_when_event_position_differs() {
    // Given: WaitUntil (event=None) and WaitEvent (timeout=None) with
    // the same text value filling the other slot
    let wait_until = StepPrimitive::Wait {
        event: None,
        timeout: Some("5".into()),
    };
    let wait_event = StepPrimitive::Wait {
        event: Some("5".into()),
        timeout: None,
    };

    // When: both are hashed independently via digest_step_primitive
    let mut hasher_until = Hasher::new();
    digest_step_primitive(&mut hasher_until, &wait_until);
    let digest_until = hasher_until.finalize();

    let mut hasher_event = Hasher::new();
    digest_step_primitive(&mut hasher_event, &wait_event);
    let digest_event = hasher_event.finalize();

    // Then: distinct digests — the positional sentinel acts as the discriminator
    assert_ne!(
        digest_until, digest_event,
        "digest_step_primitive must discriminate WaitUntil from WaitEvent via \
         positional b\"none\" sentinel (DD-4); WaitUntil hashes \
         b\"wait\"+b\"none\"+timeout while WaitEvent hashes \
         b\"wait\"+event+b\"none\""
    );
}

/// Verify the EXACT sentinel byte value `b"none"` for absent event field
/// (C3 contract). Constructs a reference hasher receiving the expected
/// byte sequence `b"wait"` + `b"none"` + timeout value, and asserts
/// `digest_step_primitive` on WaitUntil produces the identical hash.
/// Any change to the sentinel (e.g., `b"nil"`) breaks this test.
#[test]
fn digest_step_primitive_uses_exact_b_none_sentinel_when_event_is_absent() {
    // Given: WaitUntil (event=None, timeout=Some("30"))
    let wait_until = StepPrimitive::Wait {
        event: None,
        timeout: Some("30".into()),
    };

    // When: digest_step_primitive hashes the WaitUntil
    let mut hasher_until = Hasher::new();
    digest_step_primitive(&mut hasher_until, &wait_until);
    let digest_until = hasher_until.finalize();

    // Reference: manually construct the expected byte sequence
    // Wait arm order: b"wait" + event_bytes + timeout_bytes
    // event=None → b"none"
    let mut hasher_ref = Hasher::new();
    hasher_ref.update(b"wait");
    hasher_ref.update(b"none");
    hasher_ref.update(b"30");
    let digest_ref = hasher_ref.finalize();

    // Then: the WaitUntil hasher and reference must match exactly
    assert_eq!(
        digest_until, digest_ref,
        "WaitUntil (event=None) must hash exactly b\"wait\" + b\"none\" + timeout; \
         sentinel b\"none\" is the fixed C3 constant"
    );
}

/// Verify the EXACT sentinel byte value `b"none"` for absent timeout field
/// (C3 contract). Constructs a reference hasher receiving the expected
/// byte sequence `b"wait"` + event value + `b"none"`, and asserts
/// `digest_step_primitive` on WaitEvent unbounded produces the identical hash.
/// Any change to the sentinel (e.g., `b"nil"`) breaks this test.
#[test]
fn digest_step_primitive_uses_exact_b_none_sentinel_when_timeout_is_absent() {
    // Given: WaitEvent unbounded (event=Some("0"), timeout=None)
    let wait_unbounded = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: None,
    };

    // When: digest_step_primitive hashes the WaitEvent unbounded
    let mut hasher_unbounded = Hasher::new();
    digest_step_primitive(&mut hasher_unbounded, &wait_unbounded);
    let digest_unbounded = hasher_unbounded.finalize();

    // Reference: manually construct the expected byte sequence
    // Wait arm order: b"wait" + event_bytes + timeout_bytes
    // timeout=None → b"none"
    let mut hasher_ref = Hasher::new();
    hasher_ref.update(b"wait");
    hasher_ref.update(b"0");
    hasher_ref.update(b"none");
    let digest_ref = hasher_ref.finalize();

    // Then: the WaitEvent unbounded hasher and reference must match exactly
    assert_eq!(
        digest_unbounded, digest_ref,
        "WaitEvent unbounded (timeout=None) must hash exactly \
         b\"wait\" + event + b\"none\"; sentinel b\"none\" is the fixed C3 constant"
    );
}

// ---------------------------------------------------------------------------
// Section 9.2: Unit tests for `canonical_primitive_name`
// ---------------------------------------------------------------------------

/// Verify that `canonical_primitive_name` returns `"wait"` for any
/// `StepPrimitive::Wait` variant, regardless of field values.
#[test]
fn canonical_primitive_name_returns_wait_when_primitive_is_wait() {
    // Given: all three legal Wait shapes
    let wait_until = StepPrimitive::Wait {
        event: None,
        timeout: Some("5".into()),
    };
    let wait_event_unbounded = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: None,
    };
    let wait_event_bounded = StepPrimitive::Wait {
        event: Some("0".into()),
        timeout: Some("30".into()),
    };

    // When/Then: canonical_primitive_name returns "wait" for all shapes
    assert_eq!(
        canonical_primitive_name(&wait_until),
        "wait",
        "canonical_primitive_name must return 'wait' for WaitUntil"
    );
    assert_eq!(
        canonical_primitive_name(&wait_event_unbounded),
        "wait",
        "canonical_primitive_name must return 'wait' for WaitEvent unbounded"
    );
    assert_eq!(
        canonical_primitive_name(&wait_event_bounded),
        "wait",
        "canonical_primitive_name must return 'wait' for WaitEvent bounded"
    );
}

/// Verify that every `StepPrimitive` variant returns a non-empty, distinct
/// name string. This covers Set, Save, Do, Choose, ForEach, Together,
/// Collect, Aggregate, Repeat, Wait, Ask, and Finish.
#[test]
fn canonical_primitive_name_returns_non_empty_distinct_name_for_every_variant() {
    // Given: one instance of each StepPrimitive variant (using minimal dummy data)
    let variants: [(&str, StepPrimitive); 12] = [
        (
            "set",
            StepPrimitive::Set {
                output: "x".into(),
                value: "0".into(),
            },
        ),
        (
            "save",
            StepPrimitive::Save {
                value: ScalarValue::Integer(0),
            },
        ),
        (
            "do",
            StepPrimitive::Do {
                action: "act".into(),
                input: "0".into(),
            },
        ),
        (
            "choose",
            StepPrimitive::Choose {
                branches: vec![],
                otherwise: None,
            },
        ),
        (
            "for_each",
            StepPrimitive::ForEach {
                variable: "item".into(),
                input: "0".into(),
                at_once: None,
                body: vec![],
            },
        ),
        ("parallel", StepPrimitive::Together { branches: vec![] }),
        (
            "collect",
            StepPrimitive::Collect {
                variable: "page".into(),
                source: "0".into(),
                pages: None,
                items: None,
                body: vec![],
            },
        ),
        (
            "aggregate",
            StepPrimitive::Aggregate {
                variable: "acc".into(),
                input: "0".into(),
                initial: "0".into(),
                body: vec![],
            },
        ),
        (
            "repeat",
            StepPrimitive::Repeat {
                max_attempts: 3,
                body: vec![],
            },
        ),
        (
            "wait",
            StepPrimitive::Wait {
                event: None,
                timeout: Some("5".into()),
            },
        ),
        (
            "ask",
            StepPrimitive::Ask {
                prompt: "go?".into(),
                timeout: None,
            },
        ),
        (
            "finish",
            StepPrimitive::Finish {
                result: ScalarValue::Integer(0),
            },
        ),
    ];

    let mut seen_names: Vec<&str> = Vec::with_capacity(variants.len());

    for (expected_name, variant) in &variants {
        let actual = canonical_primitive_name(variant);

        // Every name must be non-empty
        assert!(
            !actual.is_empty(),
            "canonical_primitive_name must return non-empty string for {}",
            expected_name
        );

        // Every name must match the expected value
        assert_eq!(
            actual,
            *expected_name,
            "canonical_primitive_name for {:?} must return {:?}",
            std::mem::discriminant(variant),
            expected_name
        );

        // Every name must be distinct from previously seen names
        assert!(
            !seen_names.contains(&actual),
            "canonical_primitive_name must return distinct names; duplicate: '{}'",
            actual
        );

        seen_names.push(actual);
    }
}

// ---------------------------------------------------------------------------
// Section 9.4: Proptest PI-7 — Wait digest step-level idempotency
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// PI-7: Step-level idempotency.
    /// For any legal Wait field pair, calling `digest_step_primitive` twice
    /// on independent hashers produces identical finalize() outputs.
    #[test]
    fn proptest_wait_digest_step_level_idempotent(
        (event, timeout) in wait_field_strategy()
    ) {
        let wait = StepPrimitive::Wait { event, timeout };

        let mut hasher1 = Hasher::new();
        digest_step_primitive(&mut hasher1, &wait);
        let digest1 = hasher1.finalize();

        let mut hasher2 = Hasher::new();
        digest_step_primitive(&mut hasher2, &wait);
        let digest2 = hasher2.finalize();

        prop_assert_eq!(digest1, digest2,
            "digest_step_primitive must produce identical digests when called twice \
             with the same Wait input");
    }
}

// ---------------------------------------------------------------------------
// Proptest strategies for Wait field generation (mirrored from
// v1_primitive_lowering.rs to avoid cross-file coupling)
// ---------------------------------------------------------------------------

/// Generates a slot expression string: integer-like strings "0".."255".
fn wait_slot_strategy() -> impl Strategy<Value = String> {
    (0u8..=255u8).prop_map(|n| n.to_string())
}

/// Generates (Option<String>, Option<String>) pairs for wait fields.
/// At least one field will be Some (legal shape guarantee).
/// Randomly makes each field None to cover all three legal Wait shapes.
fn wait_field_strategy() -> impl Strategy<Value = (Option<String>, Option<String>)> {
    (
        wait_slot_strategy(),
        wait_slot_strategy(),
        any::<u8>(),
        any::<u8>(),
    )
        .prop_map(|(e, t, eb, tb)| {
            let event = if eb % 3 == 0 { None } else { Some(e) };
            let timeout = if tb % 3 == 0 { None } else { Some(t) };
            (event, timeout)
        })
        .prop_filter("at least one wait field must be Some", |(e, t)| {
            e.is_some() || t.is_some()
        })
}
