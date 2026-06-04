//! Unit tests for Do and Choose digest digest field sensitivity.
//! Beads: vb-qf5oj, vb-pbhor
//! These tests prove that the `digest_step_primitive` function includes
//! Do { action, input } and Choose { branches, otherwise } field bytes
//! in the hasher state — not just the canonical primitive name.
//! The bug (pre-fix): Do, Choose, and Save fell through to the `other =>`
//! catch-all which only hashed `canonical_primitive_name(other)` (e.g. b"do"
//! or b"choose"), ignoring all semantic fields.
//!
//! The fix: explicit match arms for Do, Choose, and Save that hash their
//! actual fields before any catch-all.

use super::*;
use vb_yaml::ast::{ChooseBranch, ScalarValue, StepAst, StepPrimitive};

/// Build a Do StepPrimitive.
fn do_primitive(action: &str, input: &str) -> StepPrimitive {
    StepPrimitive::Do {
        action: action.to_string(),
        input: input.to_string(),
    }
}

/// Build a minimal Choose StepPrimitive with one branch and optional otherwise.
fn choose_primitive(branches: Vec<ChooseBranch>, otherwise: Option<String>) -> StepPrimitive {
    StepPrimitive::Choose {
        branches,
        otherwise,
    }
}

/// Build a single ChooseBranch with a "when" label and a body step.
fn choose_branch(when_label: &str, body_step: StepAst) -> ChooseBranch {
    ChooseBranch {
        when: when_label.to_string(),
        steps: vec![body_step],
    }
}

/// Build a simple Set body step for use inside Choose branches.
fn set_body_step(id: &str, output: &str, value: &str) -> StepAst {
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

// ── Do digest tests ──────────────────────────────────────────────────

/// Verify Do arm hits (not catch-all fallthrough) by checking that
/// Do { action, input } produces a different digest than a catch-all
/// would produce for the same primitive name.
#[test]
fn do_action_field_affects_digest() {
    let do_step = do_primitive("my_action", "0");

    let mut hasher_explicit = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_explicit, &do_step);
    let digest_explicit = hasher_explicit.finalize();

    // Simulate the pre-fix catch-all: only hash the primitive name "do"
    let mut hasher_catch_all = blake3::Hasher::new();
    hasher_catch_all.update(b"do");
    let digest_catch_all = hasher_catch_all.finalize();

    assert_ne!(
        digest_explicit, digest_catch_all,
        "Do arm must hash field bytes beyond just the primitive name 'do'; \
         pre-fix catch-all hash checked only the name"
    );
}

/// Verify that different action values produce different digests
/// when the input is held constant.
#[test]
fn do_different_action_produces_different_digest() {
    let do_a = do_primitive("action_a", "shared_input");
    let do_b = do_primitive("action_b", "shared_input");

    let mut hasher_a = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_a, &do_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_b, &do_b);
    let digest_b = hasher_b.finalize();

    assert_ne!(
        digest_a, digest_b,
        "Do primitives with different action fields must produce different digests"
    );
}

/// Verify that different input values produce different digests
/// when the action is held constant.
#[test]
fn do_different_input_produces_different_digest() {
    let do_a = do_primitive("same_action", "input_a");
    let do_b = do_primitive("same_action", "input_b");

    let mut hasher_a = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_a, &do_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_b, &do_b);
    let digest_b = hasher_b.finalize();

    assert_ne!(
        digest_a, digest_b,
        "Do primitives with different input fields must produce different digests"
    );
}

/// Verify that Do is deterministic: calling digest_step_primitive twice
/// on the same Do primitive produces identical digests.
#[test]
fn do_deterministic_digest() {
    let do_step = do_primitive("test_action", "42");

    let mut hasher1 = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher1, &do_step);
    let digest1 = hasher1.finalize();

    let mut hasher2 = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher2, &do_step);
    let digest2 = hasher2.finalize();

    assert_eq!(
        digest1, digest2,
        "Do digest must be deterministic: identical inputs must produce identical outputs"
    );
}

// ── Choose digest tests ──────────────────────────────────────────────

/// Verify Choose arm hits (not catch-all fallthrough) by checking that
/// Choose { branches, otherwise } produces a different digest than a
/// catch-all would produce for the same primitive name.
#[test]
fn choose_fields_affect_digest() {
    let choose = choose_primitive(
        vec![choose_branch("always", set_body_step("s1", "x", "1"))],
        Some("always".to_string()),
    );

    let mut hasher_explicit = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_explicit, &choose);
    let digest_explicit = hasher_explicit.finalize();

    // Simulate the pre-fix catch-all: only hash the primitive name "choose"
    let mut hasher_catch_all = blake3::Hasher::new();
    hasher_catch_all.update(b"choose");
    let digest_catch_all = hasher_catch_all.finalize();

    assert_ne!(
        digest_explicit, digest_catch_all,
        "Choose arm must hash field bytes beyond just the primitive name 'choose'; \
         pre-fix catch-all hash checked only the name"
    );
}

/// Verify that different branch counts produce different digests.
#[test]
fn choose_different_branch_count_produces_different_digest() {
    let choose_a = choose_primitive(
        vec![choose_branch("a", set_body_step("s1", "x", "1"))],
        None,
    );
    let choose_b = choose_primitive(
        vec![
            choose_branch("a", set_body_step("s1", "x", "1")),
            choose_branch("b", set_body_step("s2", "y", "2")),
        ],
        None,
    );

    let mut hasher_a = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_a, &choose_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_b, &choose_b);
    let digest_b = hasher_b.finalize();

    assert_ne!(
        digest_a, digest_b,
        "Choose primitives with different branch counts must produce different digests"
    );
}

/// Verify that different branch labels (when conditions) produce different digests.
#[test]
fn choose_different_when_label_produces_different_digest() {
    let choose_a = choose_primitive(
        vec![choose_branch("alpha", set_body_step("s1", "x", "1"))],
        None,
    );
    let choose_b = choose_primitive(
        vec![choose_branch("beta", set_body_step("s1", "x", "1"))],
        None,
    );

    let mut hasher_a = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_a, &choose_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_b, &choose_b);
    let digest_b = hasher_b.finalize();

    assert_ne!(
        digest_a, digest_b,
        "Choose primitives with different branch 'when' labels must produce different digests"
    );
}

/// Verify that different otherwise labels produce different digests
/// when branches are held equal.
#[test]
fn choose_different_otherwise_produces_different_digest() {
    let choose_a = choose_primitive(
        vec![choose_branch("cond", set_body_step("s1", "x", "1"))],
        Some("fallback_a".to_string()),
    );
    let choose_b = choose_primitive(
        vec![choose_branch("cond", set_body_step("s1", "x", "1"))],
        Some("fallback_b".to_string()),
    );

    let mut hasher_a = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_a, &choose_a);
    let digest_a = hasher_a.finalize();

    let mut hasher_b = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_b, &choose_b);
    let digest_b = hasher_b.finalize();

    assert_ne!(
        digest_a, digest_b,
        "Choose primitives with different otherwise labels must produce different digests"
    );
}

/// Verify that None vs Some otherwise produces different digests.
#[test]
fn choose_otherwise_none_vs_some_produces_different_digest() {
    let choose_none = choose_primitive(
        vec![choose_branch("cond", set_body_step("s1", "x", "1"))],
        None,
    );
    let choose_some = choose_primitive(
        vec![choose_branch("cond", set_body_step("s1", "x", "1"))],
        Some("default".to_string()),
    );

    let mut hasher_none = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_none, &choose_none);
    let digest_none = hasher_none.finalize();

    let mut hasher_some = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher_some, &choose_some);
    let digest_some = hasher_some.finalize();

    assert_ne!(
        digest_none, digest_some,
        "Choose with no otherwise and Choose with some otherwise must produce different digests"
    );
}

/// Verify that Choose is deterministic.
#[test]
fn choose_deterministic_digest() {
    let choose = choose_primitive(
        vec![
            choose_branch("a", set_body_step("s1", "x", "1")),
            choose_branch("b", set_body_step("s2", "y", "2")),
        ],
        Some("default".to_string()),
    );

    let mut hasher1 = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher1, &choose);
    let digest1 = hasher1.finalize();

    let mut hasher2 = blake3::Hasher::new();
    let _ = digest_step_primitive(&mut hasher2, &choose);
    let digest2 = hasher2.finalize();

    assert_eq!(digest1, digest2, "Choose digest must be deterministic");
}
