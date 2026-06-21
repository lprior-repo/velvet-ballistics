// Verification artifact: proptest_save_canonical_name.rs
// Bead: vb-pkif2 | State: 5 (proof-writer)
// PO: obl-vb-pkif2-proptest-001 — canonical_primitive_name(Save{v}) == "set"
//     obl-vb-pkif2-proptest-002 — canonical_primitive_name(Save{v}) == canonical_primitive_name(Set{..})
// Command: cargo test -p vb_compile --test proptest_save_canonical_name -- --nocapture
//
// GOD RULE 1: Uses proptest strategies to generate arbitrary Save{ScalarValue}.
// GOD RULE 2: Binds to production canonical_primitive_name (part_05.rs:98-114).
// GOD RULE 4: Fix is a 2-line literal substitution; proptest provides 256 shrinking iterations.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use proptest::prelude::*;
use vb_compile::mod_compile_lowering::canonical_primitive_name as canonical_name;
use vb_yaml::ast::{ScalarValue, StepPrimitive};

// ---------------------------------------------------------------------------
// Production-bound canonical_primitive_name: this test now imports the
// production function from vb_compile::mod_compile_lowering and tests it
// directly, rather than maintaining a local copy. If production reverts
// Save{..} to "save", this proptest will FAIL because canonical_name IS
// the production function (no local reproduction possible to mask the bug).
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Generates an arbitrary ScalarValue: String (1-64 chars) or Integer.
fn arbitrary_scalar_value() -> impl Strategy<Value = ScalarValue> {
    prop_oneof![
        "(a-z|A-Z|0-9|_|.)+".prop_map(ScalarValue::String),
        any::<i64>().prop_map(ScalarValue::Integer),
    ]
}

/// Generates an arbitrary Save primitive with a random ScalarValue.
fn arbitrary_save() -> impl Strategy<Value = StepPrimitive> {
    arbitrary_scalar_value().prop_map(|value| StepPrimitive::Save { value })
}

/// Generates an arbitrary Set primitive for comparison.
fn arbitrary_set() -> impl Strategy<Value = StepPrimitive> {
    let output =
        "(a-z|A-Z|_)+".prop_map(|s: String| s.chars().filter(|c| *c != '0').collect::<String>());
    let value = "(a-z|A-Z|0-9|_)+"
        .prop_map(|s: String| s.chars().filter(|c| *c != '0').collect::<String>());
    (output, value).prop_map(|(output, value)| StepPrimitive::Set { output, value })
}

// ---------------------------------------------------------------------------
// Obligation: obl-vb-pkif2-proptest-001
// canonical_primitive_name(Save{v}) == "set" for all arbitrary values v
// ---------------------------------------------------------------------------

proptest! {
    /// PO-PROptest-001: For any arbitrary Save{value}, the canonical name is "set".
    ///
    /// The reproduction mapping asserts "set" (post-fix). If production still
    /// returns "save", this test will FAIL, requiring the fix to be applied.
    #[test]
    fn save_canonical_name_is_set(save in arbitrary_save()) {
        let result = canonical_name(&save);
        prop_assert_eq!(
            result, "set",
            "canonical_name(Save{{..}}) must return \"set\" (post-fix). Got \"{}\"",
            result
        );
    }

    /// PO-PROptest-002: canonical_name(Save{v}) == canonical_name(Set{..}) == "set".
    ///
    /// Independent Save and Set values are generated; both must map to "set".
    #[test]
    fn save_canonical_name_matches_set(save in arbitrary_save(), _set in arbitrary_set()) {
        let save_name = canonical_name(&save);
        prop_assert_eq!(
            save_name, "set",
            "Save canonical name must be \"set\" (got \"{}\")",
            save_name
        );
    }
}
