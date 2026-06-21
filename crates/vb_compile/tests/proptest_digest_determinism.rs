// Verification artifact: proptest_digest_determinism.rs
// PO: PO-PROPTEST-003 — canonical_digest is deterministic
// Bead: BH-W0-M02 (Section 38 property tests) / vb-xi2f.33
// Verifier: proptest 1.x
// Command: cargo test --test proptest_digest_determinism
//
// Proof obligations:
// - PO-PROPTEST-003: For N=500 random WorkflowSource values covering every
//   StepPrimitive variant (Set, Save, Do, Choose, ForEach, Together, Collect,
//   Reduce, Repeat, Wait, Ask, Finish) and every TriggerAst variant
//   (Manual, Schedule, Event, Webhook), canonical_digest(S) ==
//   canonical_digest(S) on every call (INV-ASK-003).
//
// GOD RULE 1: proptest strategies cover the full closed set of leaf variants;
//             no hardcoded primitive structures.
// GOD RULE 2: Binds to actual Rust canonical_digest() implementation.
// GOD RULE 3: Bounded recursion depth, no loop, deterministic proptest config.
// GOD RULE 5: Mutation thought experiment — deleting any primitive arm from
//             `arb_step_primitive_with_depth` (or its leaf helper) would
//             leave the property intact for the surviving arms but no
//             longer exercise the deleted arm's digest determinism. The
//             test still catches a regression on the surviving arms by
//             asserting both digest calls agree AND that the call returned
//             a value (Ok or Err) so a panic regression is caught.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_compile::canonical_digest;
use vb_yaml::ast::{
    ChooseBranch, ScalarValue, StepAst, StepPrimitive, TogetherBranch, TriggerAst, WorkflowSource,
    WorkflowSourceParts,
};

/// Maximum nesting depth for recursive primitives (ForEach/Together/Collect/
/// Reduce/Repeat/Choose bodies). Two levels is enough to force the canonical
/// digest to recurse into sub-step hashing at least once.
const MAX_NESTED_DEPTH: u32 = 2;

/// Number of test cases per proptest run. The strategy surface covers 12
/// primitive variants × 4 trigger variants, so 500 cases gives ≥10 of each
/// (probabilistically) with structural diversity.
const TEST_CASE_LIMIT: u32 = 500;

/// Bounded ASCII identifier for step IDs / variable names / labels.
fn arb_identifier() -> BoxedStrategy<String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,15}".boxed()
}

/// Bounded string for prompts / actions / expressions.
fn arb_text() -> BoxedStrategy<String> {
    "\\PC{0,64}".boxed()
}

/// Strategy for a `ScalarValue` (used by `Save` and `Finish`).
fn arb_scalar_value() -> BoxedStrategy<ScalarValue> {
    prop_oneof![
        arb_text().prop_map(ScalarValue::String),
        any::<i64>().prop_map(ScalarValue::Integer),
    ]
    .boxed()
}

/// Strategy for a `TriggerAst` covering all four variants.
fn arb_trigger_ast() -> BoxedStrategy<TriggerAst> {
    prop_oneof![
        Just(TriggerAst::Manual),
        arb_identifier().prop_map(|event_type| TriggerAst::Event { event_type }),
        arb_text().prop_map(|cron| TriggerAst::Schedule { cron }),
        Just(TriggerAst::Webhook),
    ]
    .boxed()
}

/// Strategy for a leaf `StepPrimitive` (no nested body).
///
/// The leaf set covers: Set, Save, Do, Wait, Ask, Finish.
fn arb_leaf_primitive() -> BoxedStrategy<StepPrimitive> {
    prop_oneof![
        (arb_identifier(), arb_text())
            .prop_map(|(output, value)| StepPrimitive::Set { output, value }),
        arb_scalar_value().prop_map(|value| StepPrimitive::Save { value }),
        (arb_identifier(), arb_text())
            .prop_map(|(action, input)| StepPrimitive::Do { action, input }),
        (prop::option::of(arb_text()), prop::option::of(arb_text()))
            .prop_map(|(event, timeout)| StepPrimitive::Wait { event, timeout }),
        (arb_text(), prop::option::of(arb_text()))
            .prop_map(|(prompt, timeout)| { StepPrimitive::Ask { prompt, timeout } }),
        arb_scalar_value().prop_map(|result| StepPrimitive::Finish { result }),
    ]
    .boxed()
}

/// Strategy for a `StepPrimitive` covering every variant. Recursive variants
/// (ForEach / Together / Collect / Reduce / Repeat / Choose) embed a body
/// of nested `StepAst`s, bounded by `MAX_NESTED_DEPTH`.
fn arb_step_primitive_with_depth(depth: u32) -> BoxedStrategy<StepPrimitive> {
    if depth == 0 {
        arb_leaf_primitive()
    } else {
        let next_depth = depth.saturating_sub(1);
        // Body steps: 0..=2 nested steps (small to keep digest computations cheap).
        let body_strategy = prop::collection::vec(arb_step_ast_with_depth(next_depth), 0..=2);

        prop_oneof![
            // ForEach: variable, input, optional at_once, body
            (
                arb_identifier(),
                arb_text(),
                prop::option::of(any::<u32>()),
                body_strategy.clone()
            )
                .prop_map(|(variable, input, at_once, body)| StepPrimitive::ForEach {
                    variable,
                    input,
                    at_once,
                    body
                }),
            // Together: branches (1..=2)
            prop::collection::vec(
                (
                    arb_identifier(),
                    prop::collection::vec(arb_step_ast_with_depth(next_depth), 0..=2)
                )
                    .prop_map(|(label, steps)| TogetherBranch { label, steps }),
                1..=2,
            )
            .prop_map(|branches| StepPrimitive::Together { branches }),
            // Collect: variable, source, optional pages, optional items, body
            (
                arb_identifier(),
                arb_text(),
                prop::option::of(any::<u32>()),
                prop::option::of(any::<u32>()),
                body_strategy.clone()
            )
                .prop_map(|(variable, source, pages, items, body)| {
                    StepPrimitive::Collect {
                        variable,
                        source,
                        pages,
                        items,
                        body,
                    }
                }),
            // Reduce: variable, input, initial, body
            (
                arb_identifier(),
                arb_text(),
                arb_text(),
                body_strategy.clone()
            )
                .prop_map(|(variable, input, initial, body)| StepPrimitive::Reduce {
                    variable,
                    input,
                    initial,
                    body,
                },),
            // Repeat: max_attempts (1..=10), body
            (1u16..=10u16, body_strategy.clone())
                .prop_map(|(max_attempts, body)| { StepPrimitive::Repeat { max_attempts, body } }),
            // Choose: 1..=2 branches, optional otherwise
            (
                prop::collection::vec(
                    (
                        arb_identifier(),
                        prop::collection::vec(arb_step_ast_with_depth(next_depth), 0..=2)
                    )
                        .prop_map(|(when, steps)| ChooseBranch { when, steps }),
                    1..=2,
                ),
                prop::option::of(arb_identifier())
            )
                .prop_map(|(branches, otherwise)| StepPrimitive::Choose {
                    branches,
                    otherwise
                }),
            // Leaf primitives (weighted by listing them three times so they
            // appear at least as often as the recursive variants).
            arb_leaf_primitive(),
            arb_leaf_primitive(),
            arb_leaf_primitive(),
        ]
        .boxed()
    }
}

/// Strategy for a single `StepAst` whose primitive is generated by
/// `arb_step_primitive_with_depth`.
fn arb_step_ast_with_depth(depth: u32) -> BoxedStrategy<StepAst> {
    (arb_identifier(), arb_step_primitive_with_depth(depth))
        .prop_map(|(id, primitive)| StepAst {
            id,
            name: None,
            condition: None,
            primitive,
            with: None,
            retry: None,
            on_error: None,
            then: None,
        })
        .boxed()
}

/// Strategy for a `WorkflowSource` with 1..=3 steps, each built from a
/// fully-typed `StepPrimitive` covering every variant.
fn workflow_source_strategy() -> BoxedStrategy<WorkflowSource> {
    (
        arb_trigger_ast(),
        arb_identifier().prop_map(|s| format!("wf_{s}")),
        prop::collection::vec(arb_step_ast_with_depth(MAX_NESTED_DEPTH), 1..=3),
    )
        .prop_map(|(trigger, name, steps)| {
            WorkflowSource::new(WorkflowSourceParts {
                version: "velvet-ballistics/v1".to_string(),
                name,
                trigger,
                inputs: vec![],
                vars: vec![],
                secrets: vec![],
                steps,
                result: None,
                examples: vec![],
            })
        })
        .boxed()
}

fn proptest_config() -> ProptestConfig {
    ProptestConfig {
        cases: TEST_CASE_LIMIT,
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(proptest_config())]

    /// PO-PROPTEST-003: Determinism property test.
    ///
    /// Same source compiled twice produces the same digest. Covers all 12
    /// `StepPrimitive` variants and all 4 `TriggerAst` variants, with
    /// recursively-nested bodies in ForEach / Together / Collect / Reduce /
    /// Repeat / Choose. The test fails on a non-deterministic digest
    /// (INV-ASK-003 violation) or a panic inside `canonical_digest`.
    #[test]
    fn prop_digest_determinism(source in workflow_source_strategy()) {
        let digest_first = canonical_digest(&source);
        let digest_second = canonical_digest(&source);
        let digest_third = canonical_digest(&source);

        // PO-PROPTEST-003: digest must agree on every call. The third call
        // catches regressions where the first two calls happen to hit a
        // cached value but a third call diverges.
        match (&digest_first, &digest_second, &digest_third) {
            (Ok(d1), Ok(d2), Ok(d3)) => {
                prop_assert_eq!(
                    d1, d2,
                    "INV-ASK-003 violated: canonical_digest is non-deterministic \
                     (run 1 vs run 2)"
                );
                prop_assert_eq!(
                    d2, d3,
                    "INV-ASK-003 violated: canonical_digest is non-deterministic \
                     (run 2 vs run 3)"
                );
            }
            (Err(_), Err(_), Err(_)) => {
                // Consistently erroring is also deterministic — a malformed
                // source (e.g. branch count overflow) returns the same Err
                // every call. No further assertion needed.
            }
            _ => {
                prop_assert!(
                    false,
                    "INV-ASK-003 violated: canonical_digest Ok/Err pattern \
                     differs across calls: {:?} vs {:?} vs {:?}",
                    digest_first, digest_second, digest_third
                );
            }
        }
    }
}
