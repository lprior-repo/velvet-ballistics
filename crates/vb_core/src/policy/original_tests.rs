//! Original seven RuntimePolicy tests, preserved verbatim from base commit.

use crate::policy::RuntimePolicy;

#[test]
fn policy_variants_are_distinct() {
    assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
    assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
    assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
}

#[test]
fn policy_copy_semantics_preserve_equality() {
    let a = RuntimePolicy::Strict;
    let b = a;
    assert_eq!(a, b, "copy must preserve equality");
}

#[test]
fn policy_strict_is_not_journaled() {
    assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Journaled);
}

#[test]
fn policy_strict_is_not_relaxed() {
    assert_ne!(RuntimePolicy::Strict, RuntimePolicy::Relaxed);
}

#[test]
fn policy_journaled_is_not_relaxed() {
    assert_ne!(RuntimePolicy::Journaled, RuntimePolicy::Relaxed);
}

#[test]
fn policy_debug_output_contains_variant_name() {
    let formatted = format!("{:?}", RuntimePolicy::Strict);
    assert!(
        formatted.contains("Strict"),
        "debug output must contain variant name: {formatted}"
    );
}

#[test]
fn policy_clone_produces_equal_value() {
    let original = RuntimePolicy::Journaled;
    let cloned = original.clone();
    assert_eq!(original, cloned, "clone must produce equal value");
}
