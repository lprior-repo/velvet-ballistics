use vb_core::{Taint, join_taint};

use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributorTaints {
    values: Vec<Taint>,
}

impl ContributorTaints {
    pub fn try_new(values: Vec<Taint>) -> RuntimeResult<Self> {
        if values.is_empty() {
            Err(RuntimeError::InvalidRecoveryHydration)
        } else {
            Ok(Self { values })
        }
    }

    fn as_slice(&self) -> &[Taint] {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolvedNodeTaintInput {
    EvalExpr { contributor_taints: ContributorTaints },
    BuildObject { contributor_taints: ContributorTaints },
    BuildList { contributor_taints: ContributorTaints },
    Finish { result_taint: Taint },
}

impl ResolvedNodeTaintInput {
    pub fn eval_expr(contributor_taints: Vec<Taint>) -> RuntimeResult<Self> {
        Ok(Self::EvalExpr {
            contributor_taints: ContributorTaints::try_new(contributor_taints)?,
        })
    }

    pub fn build_object(contributor_taints: Vec<Taint>) -> RuntimeResult<Self> {
        Ok(Self::BuildObject {
            contributor_taints: ContributorTaints::try_new(contributor_taints)?,
        })
    }

    pub fn build_list(contributor_taints: Vec<Taint>) -> RuntimeResult<Self> {
        Ok(Self::BuildList {
            contributor_taints: ContributorTaints::try_new(contributor_taints)?,
        })
    }

    pub fn finish(result_taint: Taint) -> Self {
        Self::Finish { result_taint }
    }
}

pub fn resolved_node_output_taint(input: &ResolvedNodeTaintInput) -> RuntimeResult<Taint> {
    match input {
        ResolvedNodeTaintInput::EvalExpr { contributor_taints }
        | ResolvedNodeTaintInput::BuildObject { contributor_taints }
        | ResolvedNodeTaintInput::BuildList { contributor_taints } => {
            Ok(join_all(contributor_taints.as_slice().iter().copied()))
        }
        ResolvedNodeTaintInput::Finish { result_taint } => Ok(*result_taint),
    }
}

fn join_all<I>(taints: I) -> Taint
where I: IntoIterator<Item = Taint>,
{
    taints.into_iter().fold(Taint::Clean, join_taint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::Taint;

    #[test]
    fn contributor_taints_try_new_succeeds_with_single_taint() {
        let result = ContributorTaints::try_new(vec![Taint::Clean]);
        assert_eq!(result, Ok(ContributorTaints { values: vec![Taint::Clean] }));
    }

    #[test]
    fn contributor_taints_try_new_succeeds_with_multiple_taints() {
        let result = ContributorTaints::try_new(vec![Taint::Clean, Taint::Secret, Taint::Random]);
        assert_eq!(result, Ok(ContributorTaints {
            values: vec![Taint::Clean, Taint::Secret, Taint::Random],
        }));
    }

    #[test]
    fn contributor_taints_try_new_fails_with_empty_input() {
        let result = ContributorTaints::try_new(vec![]);
        assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
    }

    #[test]
    fn contributor_taints_try_new_preserves_order() {
        let result = ContributorTaints::try_new(vec![
            Taint::Secret, Taint::DerivedFromSecret, Taint::TimeDependent,
        ]);
        let Ok(container) = result else { panic!("expected Ok"); };
        match container.values.as_slice() {
            [Taint::Secret, Taint::DerivedFromSecret, Taint::TimeDependent] => {},
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn eval_expr_succeeds_with_valid_contributors() {
        let result = ResolvedNodeTaintInput::eval_expr(vec![Taint::Clean]);
        match result {
            Ok(ResolvedNodeTaintInput::EvalExpr { contributor_taints }) => {
                assert_eq!(contributor_taints.values, vec![Taint::Clean]);
            }
            other => panic!("expected Ok(EvalExpr), got {other:?}"),
        }
    }

    #[test]
    fn eval_expr_fails_with_empty_contributors() {
        let result = ResolvedNodeTaintInput::eval_expr(vec![]);
        assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
    }

    #[test]
    fn build_object_succeeds_with_valid_contributors() {
        let result = ResolvedNodeTaintInput::build_object(vec![Taint::Secret, Taint::DerivedFromSecret]);
        match result {
            Ok(ResolvedNodeTaintInput::BuildObject { contributor_taints }) => {
                assert_eq!(contributor_taints.values, vec![Taint::Secret, Taint::DerivedFromSecret]);
            }
            other => panic!("expected Ok(BuildObject), got {other:?}"),
        }
    }

    #[test]
    fn build_object_fails_with_empty_contributors() {
        let result = ResolvedNodeTaintInput::build_object(vec![]);
        assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
    }

    #[test]
    fn build_list_succeeds_with_valid_contributors() {
        let result = ResolvedNodeTaintInput::build_list(vec![Taint::Random]);
        match result {
            Ok(ResolvedNodeTaintInput::BuildList { contributor_taints }) => {
                assert_eq!(contributor_taints.values, vec![Taint::Random]);
            }
            other => panic!("expected Ok(BuildList), got {other:?}"),
        }
    }

    #[test]
    fn build_list_fails_with_empty_contributors() {
        let result = ResolvedNodeTaintInput::build_list(vec![]);
        assert_eq!(result, Err(RuntimeError::InvalidRecoveryHydration));
    }

    #[test]
    fn finish_creates_with_taint() {
        let result = ResolvedNodeTaintInput::finish(Taint::DerivedFromSecret);
        assert_eq!(result, ResolvedNodeTaintInput::Finish { result_taint: Taint::DerivedFromSecret });
    }

    #[test]
    fn finish_creates_with_clean_taint() {
        let result = ResolvedNodeTaintInput::finish(Taint::Clean);
        assert_eq!(result, ResolvedNodeTaintInput::Finish { result_taint: Taint::Clean });
    }

    #[test]
    fn resolved_node_output_taint_eval_expr_all_clean_returns_clean() {
        let input = ResolvedNodeTaintInput::eval_expr(vec![Taint::Clean, Taint::Clean]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Clean));
    }

    #[test]
    fn resolved_node_output_taint_eval_expr_with_secret_returns_secret() {
        let input = ResolvedNodeTaintInput::eval_expr(vec![Taint::Clean, Taint::Secret]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Secret));
    }

    #[test]
    fn resolved_node_output_taint_build_object_with_derived_returns_derived() {
        let input = ResolvedNodeTaintInput::build_object(vec![Taint::DerivedFromSecret]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::DerivedFromSecret));
    }

    #[test]
    fn resolved_node_output_taint_build_list_multiple_taints_joins() {
        let input = ResolvedNodeTaintInput::build_list(vec![
            Taint::Clean, Taint::TimeDependent, Taint::Clean,
        ]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::TimeDependent));
    }

    #[test]
    fn resolved_node_output_taint_finish_preserves_taint_exactly() {
        let input = ResolvedNodeTaintInput::finish(Taint::Random);
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Random));
    }

    #[test]
    fn resolved_node_output_taint_finish_clean_returns_clean() {
        let input = ResolvedNodeTaintInput::finish(Taint::Clean);
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Clean));
    }

    #[test]
    fn resolved_node_taint_input_variants_are_distinct() {
        let eval = ResolvedNodeTaintInput::eval_expr(vec![Taint::Clean]).unwrap();
        let obj = ResolvedNodeTaintInput::build_object(vec![Taint::Clean]).unwrap();
        let list = ResolvedNodeTaintInput::build_list(vec![Taint::Clean]).unwrap();
        let fin = ResolvedNodeTaintInput::finish(Taint::Clean);
        assert_ne!(eval, obj);
        assert_ne!(eval, list);
        assert_ne!(eval, fin);
        assert_ne!(obj, list);
    }

    #[test]
    fn resolved_node_taint_input_same_variant_same_data_equals() {
        let a = ResolvedNodeTaintInput::finish(Taint::Secret);
        let b = ResolvedNodeTaintInput::finish(Taint::Secret);
        assert_eq!(a, b);
    }

    #[test]
    fn resolved_node_taint_input_finish_different_taint_not_equal() {
        assert_ne!(
            ResolvedNodeTaintInput::finish(Taint::Clean),
            ResolvedNodeTaintInput::finish(Taint::Secret),
        );
    }

    #[test]
    fn contributor_taints_debug_format_contains_values() {
        let taints = ContributorTaints::try_new(vec![Taint::Clean]).unwrap();
        let debug = format!("{taints:?}");
        assert!(debug.contains("Clean"), "debug: {debug}");
    }

    #[test]
    fn resolved_node_output_taint_eval_expr_single_random_returns_random() {
        let input = ResolvedNodeTaintInput::eval_expr(vec![Taint::Random]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Random));
    }

    #[test]
    fn resolved_node_output_taint_eval_expr_single_time_dependent_returns_time_dependent() {
        let input = ResolvedNodeTaintInput::eval_expr(vec![Taint::TimeDependent]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::TimeDependent));
    }

    #[test]
    fn resolved_node_output_taint_eval_expr_all_secret_returns_secret() {
        let input = ResolvedNodeTaintInput::eval_expr(vec![Taint::Secret, Taint::Secret]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Secret));
    }

    #[test]
    fn resolved_node_output_taint_build_object_all_clean_returns_clean() {
        let input = ResolvedNodeTaintInput::build_object(vec![
            Taint::Clean, Taint::Clean, Taint::Clean,
        ]).unwrap();
        let result = resolved_node_output_taint(&input);
        assert_eq!(result, Ok(Taint::Clean));
    }
}
