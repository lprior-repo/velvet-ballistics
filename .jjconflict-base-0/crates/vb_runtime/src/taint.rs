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
pub enum ResolvedNodeTaintInput {
    EvalExpr {
        contributor_taints: ContributorTaints,
    },
    BuildObject {
        contributor_taints: ContributorTaints,
    },
    BuildList {
        contributor_taints: ContributorTaints,
    },
    Finish {
        result_taint: Taint,
    },
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
where
    I: IntoIterator<Item = Taint>,
{
    taints.into_iter().fold(Taint::Clean, join_taint)
}
