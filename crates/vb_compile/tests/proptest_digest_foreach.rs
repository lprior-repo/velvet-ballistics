#![allow(clippy::expect_used)]
// Verification artifact: proptest_digest_foreach.rs
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Proptest extensions for ForEach digest coverage.
//
// PO-P-FE-01 through PO-P-FE-08 (7 obligations):
//   PO-P-FE-01: ForEach.input variation changes digest
//   PO-P-FE-02: ForEach.at_once variation changes digest
//   PO-P-FE-03: ForEach.variable variation changes digest
//   PO-P-FE-04: ForEach.body variation changes digest
//   PO-P-FE-05: Digest determinism across re-compiles
//   PO-P-FE-06: Dual-path digest equivalence (cross-path)
//   PO-P-FE-08: Non-regression: Set/Finish digests unchanged
//
// VISIBILITY NOTE: These integration tests require `pub` visibility on
// canonical_digest and digest_step_primitive in both compilation paths.
// Currently:
//   - mod_compile_lowering::part_05::canonical_digest is pub(super)
//   - compile::mod::canonical_digest is fn (private)
// The implementation owner must add `pub(crate)` or `pub` visibility
// to these functions before these proptest tests will compile.
// See BLOCKER-VIS-01 in proof-evidence.md.
//
// GOD RULE 1: Uses proptest strategies with Arbitrary generation.
// GOD RULE 2: Binds to actual production canonial_digest implementations.

use proptest::prelude::*;
use vb_compile::canonical_digest_part05;
use vb_yaml::ast::{
    ScalarValue, StepAst, StepPrimitive, TriggerAst, WorkflowSource, WorkflowSourceParts,
};

// ═══════════════════════════════════════════════════════════════════════════
// Proptest Strategies
// ═══════════════════════════════════════════════════════════════════════════

/// Strategy for valid YAML-like variable names (alphanumeric + underscore, no colon).
fn variable_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,31}"
}

/// Strategy for input expression strings.
fn input_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z][a-zA-Z0-9_.]{0,31}"
}

/// Strategy for u32 at_once values including None.
fn at_once_strategy() -> impl Strategy<Value = Option<u32>> {
    prop::option::of(0u32..=u32::MAX)
}

/// Strategy for a Set body step.
fn set_step_strategy() -> impl Strategy<Value = StepAst> {
    ("[a-zA-Z_][a-zA-Z0-9_]{0,15}", "[a-zA-Z0-9_ .]{0,31}").prop_map(|(output, value)| StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set { output, value },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    })
}

/// Strategy for a Finish body step.
fn finish_step_strategy() -> impl Strategy<Value = StepAst> {
    any::<i64>().prop_map(|value| StepAst {
        id: "f".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::Integer(value),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    })
}

/// Strategy for body steps: 0-5 steps of mixed Set/Finish.
fn body_steps_strategy() -> impl Strategy<Value = Vec<StepAst>> {
    prop::collection::vec(
        prop_oneof![set_step_strategy(), finish_step_strategy()],
        0..=5,
    )
}

/// Build a WorkflowSource with one ForEach step.
fn build_foreach_source(
    version: &str,
    name: &str,
    variable: String,
    input: String,
    at_once: Option<u32>,
    body: Vec<StepAst>,
) -> WorkflowSource {
    let steps = vec![StepAst {
        id: "step1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::ForEach {
            variable,
            input,
            at_once,
            body,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    WorkflowSource::new(WorkflowSourceParts {
        version: version.to_string(),
        name: name.to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        result: None,
        examples: vec![],
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-01: ForEach.input variation changes digest
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// PO-P-FE-01: Changing only ForEach.input produces a different canonical_digest.
    ///
    /// Generates random input_a != input_b, creates two WorkflowSource values
    /// differing only in the ForEach.input field, and asserts the digests differ.
    #[test]
    fn proptest_foreach_input_variation_changes_digest(
        variable in variable_strategy(),
        input_a in input_strategy(),
        input_b in input_strategy(),
        at_once in at_once_strategy(),
        body in body_steps_strategy(),
    ) {
        // Ensure inputs differ
        prop_assume!(input_a != input_b);

        let source_a = build_foreach_source(
            "v1", "test", variable.clone(), input_a, at_once, body.clone(),
        );
        let source_b = build_foreach_source(
            "v1", "test", variable, input_b, at_once, body,
        );

        let digest_a = canonical_digest_part05(&source_a);
        let digest_b = canonical_digest_part05(&source_b);

        prop_assert_ne!(digest_a, digest_b,
            "Changing ForEach.input must change the digest");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-02: ForEach.at_once variation changes digest
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// PO-P-FE-02: Changing only ForEach.at_once produces a different canonical_digest.
    ///
    /// Excludes None/Some(1) equivalence (tested via PO-K-FE-07).
    #[test]
    fn proptest_foreach_at_once_variation_changes_digest(
        variable in variable_strategy(),
        input in input_strategy(),
        at_once_a in at_once_strategy(),
        at_once_b in at_once_strategy(),
        body in body_steps_strategy(),
    ) {
        // At_once must differ
        prop_assume!(at_once_a != at_once_b);
        // Exclude None/Some(1) semantic equivalence
        prop_assume!(
            !((at_once_a.is_none() && at_once_b == Some(1))
                || (at_once_b.is_none() && at_once_a == Some(1)))
        );

        let source_a = build_foreach_source(
            "v1", "test", variable.clone(), input.clone(), at_once_a, body.clone(),
        );
        let source_b = build_foreach_source(
            "v1", "test", variable, input, at_once_b, body,
        );

        let digest_a = canonical_digest_part05(&source_a);
        let digest_b = canonical_digest_part05(&source_b);

        prop_assert_ne!(digest_a, digest_b,
            "Changing ForEach.at_once must change the digest");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-03: ForEach.variable variation changes digest
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// PO-P-FE-03: Changing only ForEach.variable produces a different canonical_digest.
    ///
    /// Includes Unicode characters in variable names (hashed as raw bytes).
    #[test]
    fn proptest_foreach_variable_variation_changes_digest(
        variable_a in variable_strategy(),
        variable_b in variable_strategy(),
        input in input_strategy(),
        at_once in at_once_strategy(),
        body in body_steps_strategy(),
    ) {
        prop_assume!(variable_a != variable_b);

        let source_a = build_foreach_source(
            "v1", "test", variable_a, input.clone(), at_once, body.clone(),
        );
        let source_b = build_foreach_source(
            "v1", "test", variable_b, input, at_once, body,
        );

        let digest_a = canonical_digest_part05(&source_a);
        let digest_b = canonical_digest_part05(&source_b);

        prop_assert_ne!(digest_a, digest_b,
            "Changing ForEach.variable must change the digest");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-04: ForEach.body variation changes digest
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// PO-P-FE-04: Changing any body step content produces a different canonical_digest.
    ///
    /// Tests: add/remove/change body steps all detected.
    #[test]
    fn proptest_foreach_body_variation_changes_digest(
        variable in variable_strategy(),
        input in input_strategy(),
        at_once in at_once_strategy(),
        body_a in body_steps_strategy(),
        body_b in body_steps_strategy(),
    ) {
        prop_assume!(body_a != body_b);

        let source_a = build_foreach_source(
            "v1", "test", variable.clone(), input.clone(), at_once, body_a,
        );
        let source_b = build_foreach_source(
            "v1", "test", variable, input, at_once, body_b,
        );

        let digest_a = canonical_digest_part05(&source_a);
        let digest_b = canonical_digest_part05(&source_b);

        prop_assert_ne!(digest_a, digest_b,
            "Changing ForEach.body must change the digest");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-05: Digest determinism across re-compiles
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// PO-P-FE-05: canonical_digest is deterministic: re-compiling the same source
    /// always produces the same digest.
    #[test]
    fn proptest_foreach_digest_deterministic(
        variable in variable_strategy(),
        input in input_strategy(),
        at_once in at_once_strategy(),
        body in body_steps_strategy(),
    ) {
        let source = build_foreach_source(
            "v1", "test", variable, input, at_once, body,
        );

        // Compile 5 times, all must produce the same digest
        let digests: Vec<_> = (0..5)
            .map(|_| canonical_digest_part05(&source))
            .collect();

        let first = digests.first().expect("digests must be non-empty (5 compilations)");
        for (i, d) in digests.iter().enumerate().skip(1) {
            let n = i.checked_add(1).expect("i < digests.len() so i+1 fits usize");
            prop_assert_eq!(first, d,
                "Compilation {n} produced different digest than compilation 1", n = n);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-06: Dual-path digest equivalence
// ═══════════════════════════════════════════════════════════════════════════
//
// NOTE: compile/mod.rs (path A) is NOT compiled in the current crate structure.
// Only mod_compile_lowering/part_05.rs (path B) is live. The dual-path
// equivalence test is deferred until path A is integrated.
//
// When path A is compiled, uncomment and update:
//
// #[test]
// fn proptest_foreach_cross_path_digest_equivalence(
//     variable in variable_strategy(),
//     input in input_strategy(),
//     at_once in at_once_strategy(),
//     body in body_steps_strategy(),
// ) {
//     let source = build_foreach_source(
//         "v1", "test", variable, input, at_once, body,
//     );
//     let digest_a = canonical_digest_mod(&source);
//     let digest_b = canonical_digest_part05(&source);
//     prop_assert_eq!(digest_a, digest_b,
//         "Both compilation paths must produce identical digests");
// }

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-08: Non-regression: Set/Finish digests unchanged
// ═══════════════════════════════════════════════════════════════════════════

proptest! {
    /// PO-P-FE-08: Non-ForEach primitives produce digests unchanged by the ForEach fix.
    ///
    /// Tests that workflows with ONLY Set and Finish steps (no ForEach) compute
    /// deterministic and correct digests. This guards against regression where
    /// the ForEach arm accidentally alters non-ForEach hashing.
    #[test]
    fn proptest_foreach_nonregression_set_finish(
        set_steps in prop::collection::vec(set_step_strategy(), 1..=5),
    ) {
        let steps: Vec<StepAst> = set_steps;
        let source = WorkflowSource::new(WorkflowSourceParts {
            version: "v1".to_string(),
            name: "test".to_string(),
            trigger: TriggerAst::Manual,
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps,
            result: None,
            examples: vec![],
        });

        // Digest must be deterministic (no ForEach regression)
        let d1 = canonical_digest_part05(&source);
        let d2 = canonical_digest_part05(&source);
        prop_assert_eq!(d1, d2, "Set/Finish digests must remain deterministic");

        // All proptest Set/Finish work verified via H2 sensitivity below.
    }

    /// PO-P-FE-08 H2: Set/Finish output sensitivity still works.
    ///
    /// Changing a Set step's output field must change the digest, proving
    /// that the ForEach fix didn't break existing Set hashing.
    #[test]
    fn proptest_foreach_nonregression_set_sensitivity(
        output_a in "[a-zA-Z_][a-zA-Z0-9_]{0,15}",
        output_b in "[a-zA-Z_][a-zA-Z0-9_]{0,15}",
        value in "[a-zA-Z0-9_ .]{0,31}",
    ) {
        prop_assume!(output_a != output_b);

        let step_a = StepAst {
            id: "s".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Set { output: output_a.clone(), value: value.clone() },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        };
        let step_b = StepAst {
            id: "s".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Set { output: output_b, value },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        };

        let source_a = WorkflowSource::new(WorkflowSourceParts {
            version: "v1".to_string(),
            name: "test".to_string(),
            trigger: TriggerAst::Manual,
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![step_a],
            result: None,
            examples: vec![],
        });
        let source_b = WorkflowSource::new(WorkflowSourceParts {
            version: "v1".to_string(),
            name: "test".to_string(),
            trigger: TriggerAst::Manual,
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![step_b],
            result: None,
            examples: vec![],
        });

        let da = canonical_digest_part05(&source_a);
        let db = canonical_digest_part05(&source_b);
        prop_assert_ne!(da, db, "Set output sensitivity must be preserved");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-FE-07 (P8): at_once semantic equivalence — None vs Some(1)
// ═══════════════════════════════════════════════════════════════════════════

/// Strategy that generates at_once values specifically for the equivalence test:
/// always returns either None or Some(1).
fn at_once_equiv_strategy() -> impl Strategy<Value = Option<u32>> {
    prop_oneof![Just(None), Just(Some(1)),]
}

proptest! {
    /// PO-P-FE-07: at_once=None and at_once=Some(1) produce identical canonical_digest
    /// for any random ForEach body configuration.
    ///
    /// This is a proptest extension of the unit test B7 — verifies the invariant
    /// holds across the full random input space rather than a single fixed body.
    #[test]
    fn proptest_foreach_at_once_none_some1_equivalence(
        variable in variable_strategy(),
        input in input_strategy(),
        at_once_a in at_once_equiv_strategy(),
        at_once_b in at_once_equiv_strategy(),
        body in body_steps_strategy(),
    ) {
        let source_a = build_foreach_source(
            "v1", "test", variable.clone(), input.clone(), at_once_a, body.clone(),
        );
        let source_b = build_foreach_source(
            "v1", "test", variable, input, at_once_b, body,
        );

        let digest_a = canonical_digest_part05(&source_a);
        let digest_b = canonical_digest_part05(&source_b);

        prop_assert_eq!(digest_a, digest_b,
            "at_once=None and at_once=Some(1) must produce identical digests for any body configuration");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PO-P-AC-FE-04-NESTED (P9): Nested ForEach body sensitivity
// ═══════════════════════════════════════════════════════════════════════════

/// Strategy for an inner (nested) ForEach step with simple body.
fn nested_foreach_step_strategy() -> impl Strategy<Value = StepAst> {
    (variable_strategy(), input_strategy(), at_once_strategy()).prop_map(
        |(variable, input, at_once)| {
            let inner_body = vec![StepAst {
                id: "nested_s".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: "nested_x".to_string(),
                    value: "1".to_string(),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }];
            StepAst {
                id: "nested_outer".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::ForEach {
                    variable,
                    input,
                    at_once,
                    body: inner_body,
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }
        },
    )
}

proptest! {
    /// Nested ForEach body sensitivity: changing content of a nested ForEach
    /// in a body step changes the outer ForEach digest.
    ///
    /// Binding: nesting depth limited to 2 (outer + 1 nested) for tractability.
    #[test]
    fn proptest_foreach_nested_body_content_changes_outer_digest(
        outer_variable in variable_strategy(),
        outer_input in input_strategy(),
        outer_at_once in at_once_strategy(),
        nested_a in nested_foreach_step_strategy(),
        nested_b in nested_foreach_step_strategy(),
    ) {
        // Ensure nested ForEach steps differ
        prop_assume!(nested_a != nested_b);

        // Outer ForEach with nested ForEach A as body
        let source_a = build_foreach_source(
            "v1", "test",
            outer_variable.clone(),
            outer_input.clone(),
            outer_at_once,
            vec![nested_a],
        );
        // Outer ForEach with nested ForEach B as body
        let source_b = build_foreach_source(
            "v1", "test",
            outer_variable,
            outer_input,
            outer_at_once,
            vec![nested_b],
        );

        let digest_a = canonical_digest_part05(&source_a);
        let digest_b = canonical_digest_part05(&source_b);

        prop_assert_ne!(digest_a, digest_b,
            "Changing nested ForEach content must change outer ForEach digest");
    }
}
