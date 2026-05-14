use proptest::prelude::*;
use vb_core::{Taint, join_taint};
use vb_runtime::{
    RuntimeError, RuntimeResult,
    taint::{ContributorTaints, ResolvedNodeTaintInput, resolved_node_output_taint},
};

pub mod taint {
    pub mod proptests {
        use super::super::*;

        proptest! {
            #[test]
            fn eval_expr_joined_taint_propagation(contributors in contributor_discriminants()) {
                let taints = taints_from_discriminants(contributors);
                let expected = join_all(taints.iter().copied());
                let result = output_for(ResolvedNodeTaintInput::eval_expr(taints));
                prop_assert_eq!(result, Ok(expected));
            }

            #[test]
            fn build_object_joined_taint_propagation(contributors in contributor_discriminants()) {
                let taints = taints_from_discriminants(contributors);
                let expected = join_all(taints.iter().copied());
                let result = output_for(ResolvedNodeTaintInput::build_object(taints));
                prop_assert_eq!(result, Ok(expected));
            }

            #[test]
            fn build_list_joined_taint_propagation(contributors in contributor_discriminants()) {
                let taints = taints_from_discriminants(contributors);
                let expected = join_all(taints.iter().copied());
                let result = output_for(ResolvedNodeTaintInput::build_list(taints));
                prop_assert_eq!(result, Ok(expected));
            }
        }
    }
}

#[test]
fn eval_expr_rejects_empty_contributor_domain() {
    assert_eq!(
        ResolvedNodeTaintInput::eval_expr(Vec::new()),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn build_object_rejects_empty_contributor_domain() {
    assert_eq!(
        ResolvedNodeTaintInput::build_object(Vec::new()),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn build_list_rejects_empty_contributor_domain() {
    assert_eq!(
        ResolvedNodeTaintInput::build_list(Vec::new()),
        Err(RuntimeError::InvalidRecoveryHydration)
    );
}

#[test]
fn contributor_taints_try_new_rejects_empty_domain() {
    let result = ContributorTaints::try_new(Vec::new());

    assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
}

#[test]
fn contributor_taints_try_new_allows_single_clean_value() {
    let result = ContributorTaints::try_new(vec![Taint::Clean]).map(|contributor_taints| {
        let input = ResolvedNodeTaintInput::EvalExpr { contributor_taints };
        resolved_node_output_taint(&input)
    });

    assert_eq!(result, Ok(Ok(Taint::Clean)));
}

#[test]
fn contributor_taints_try_new_allows_single_secret_value() {
    let result = ContributorTaints::try_new(vec![Taint::Secret]).map(|contributor_taints| {
        let input = ResolvedNodeTaintInput::BuildObject { contributor_taints };
        resolved_node_output_taint(&input)
    });

    assert_eq!(result, Ok(Ok(Taint::Secret)));
}

#[test]
fn contributor_taints_try_new_preserves_join_across_three_values() {
    let result =
        ContributorTaints::try_new(vec![Taint::Clean, Taint::DerivedFromSecret, Taint::Secret])
            .map(|contributor_taints| {
                let input = ResolvedNodeTaintInput::BuildList { contributor_taints };
                resolved_node_output_taint(&input)
            });

    assert_eq!(result, Ok(Ok(Taint::Secret)));
}

#[test]
fn eval_expr_output_taint_returns_secret_for_derived_and_secret_contributors() {
    let result = output_for(ResolvedNodeTaintInput::eval_expr(vec![
        Taint::DerivedFromSecret,
        Taint::Secret,
    ]));

    assert_eq!(result, Ok(Taint::Secret));
}

#[test]
fn eval_expr_output_taint_returns_derived_for_single_derived_contributor() {
    let result = output_for(ResolvedNodeTaintInput::eval_expr(vec![
        Taint::DerivedFromSecret,
    ]));

    assert_eq!(result, Ok(Taint::DerivedFromSecret));
}

#[test]
fn eval_expr_output_taint_returns_secret_for_clean_and_secret_contributors() {
    let result = output_for(ResolvedNodeTaintInput::eval_expr(vec![
        Taint::Clean,
        Taint::Secret,
    ]));

    assert_eq!(result, Ok(Taint::Secret));
}

#[test]
fn build_object_output_taint_returns_derived_for_clean_and_derived_contributors() {
    let result = output_for(ResolvedNodeTaintInput::build_object(vec![
        Taint::Clean,
        Taint::DerivedFromSecret,
    ]));

    assert_eq!(result, Ok(Taint::DerivedFromSecret));
}

#[test]
fn build_object_output_taint_returns_clean_for_single_clean_contributor() {
    let result = output_for(ResolvedNodeTaintInput::build_object(vec![Taint::Clean]));

    assert_eq!(result, Ok(Taint::Clean));
}

#[test]
fn build_object_output_taint_returns_secret_for_clean_and_secret_contributors() {
    let result = output_for(ResolvedNodeTaintInput::build_object(vec![
        Taint::Clean,
        Taint::Secret,
    ]));

    assert_eq!(result, Ok(Taint::Secret));
}

#[test]
fn build_object_output_taint_returns_secret_for_all_contributor_levels() {
    let result = output_for(ResolvedNodeTaintInput::build_object(vec![
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
    ]));

    assert_eq!(result, Ok(Taint::Secret));
}

#[test]
fn build_list_output_taint_returns_clean_for_single_clean_contributor() {
    let result = output_for(ResolvedNodeTaintInput::build_list(vec![Taint::Clean]));

    assert_eq!(result, Ok(Taint::Clean));
}

#[test]
fn build_list_output_taint_returns_derived_for_single_derived_contributor() {
    let result = output_for(ResolvedNodeTaintInput::build_list(vec![
        Taint::DerivedFromSecret,
    ]));

    assert_eq!(result, Ok(Taint::DerivedFromSecret));
}

#[test]
fn build_list_output_taint_returns_secret_for_clean_and_secret_contributors() {
    let result = output_for(ResolvedNodeTaintInput::build_list(vec![
        Taint::Clean,
        Taint::Secret,
    ]));

    assert_eq!(result, Ok(Taint::Secret));
}

#[test]
fn build_list_output_taint_returns_secret_for_all_contributor_levels() {
    let result = output_for(ResolvedNodeTaintInput::build_list(vec![
        Taint::Clean,
        Taint::DerivedFromSecret,
        Taint::Secret,
    ]));

    assert_eq!(result, Ok(Taint::Secret));
}

#[test]
fn finish_output_taint_preserves_clean_result_taint() {
    let result = resolved_node_output_taint(&ResolvedNodeTaintInput::finish(Taint::Clean));

    assert_eq!(result, Ok(Taint::Clean));
}

#[test]
fn finish_output_taint_preserves_derived_result_taint() {
    let result =
        resolved_node_output_taint(&ResolvedNodeTaintInput::finish(Taint::DerivedFromSecret));

    assert_eq!(result, Ok(Taint::DerivedFromSecret));
}

#[test]
fn finish_output_taint_preserves_secret_result_taint() {
    let result = resolved_node_output_taint(&ResolvedNodeTaintInput::finish(Taint::Secret));

    assert_eq!(result, Ok(Taint::Secret));
}

fn contributor_discriminants() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0_u8..=2, 1..=4)
}

fn taints_from_discriminants(values: Vec<u8>) -> Vec<Taint> {
    values.into_iter().map(taint_from_discriminant).collect()
}

fn taint_from_discriminant(value: u8) -> Taint {
    match value {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        _ => Taint::Secret,
    }
}

fn output_for(input: RuntimeResult<ResolvedNodeTaintInput>) -> RuntimeResult<Taint> {
    input.and_then(|node| resolved_node_output_taint(&node))
}

fn join_all<I>(taints: I) -> Taint
where
    I: IntoIterator<Item = Taint>,
{
    taints.into_iter().fold(Taint::Clean, join_taint)
}
