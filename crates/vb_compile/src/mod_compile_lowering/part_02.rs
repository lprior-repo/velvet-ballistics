use super::part_01::ForEachInputs;
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

#[cfg(test)]
pub(crate) use super::part_14::lower_canonical_choose;

#[path = "canonical_scope.rs"]
mod canonical_scope;
pub(super) use canonical_scope::{CanonicalScope, validate_output_alias};

#[path = "canonical_references.rs"]
mod canonical_references;
pub(super) use canonical_references::{resolve_slot_reference, resolve_slot_reference_scoped};
#[path = "for_each_capture.rs"]
pub(super) mod for_each_capture;

#[path = "canonical_types.rs"]
mod canonical_types;

#[path = "canonical_scalar.rs"]
mod canonical_scalar;
pub(super) use canonical_scalar::{
    compile_canonical_expression, lower_canonical_set_expression,
    lower_canonical_set_expression_with_binding,
};

#[path = "canonical_do.rs"]
mod canonical_do;

#[path = "canonical_controls.rs"]
pub(super) mod canonical_controls;

#[path = "canonical_guarded_do.rs"]
mod canonical_guarded_do;
use canonical_guarded_do::lower_top_level_do;

#[path = "bounded_for_each.rs"]
mod bounded_for_each;
use bounded_for_each::{ForEachLowering, lower_bounded_for_each};

#[path = "bounded_collect.rs"]
pub(super) mod bounded_collect;
use bounded_collect::{CollectLowering, lower_bounded_collect};

#[path = "for_each_strict.rs"]
mod for_each_strict;
pub(super) use for_each_strict::validate_for_each_expression;

#[path = "canonical_step.rs"]
mod step;
pub(super) use step::lower_canonical_step;

#[path = "canonical_finish.rs"]
mod canonical_finish;
pub(super) use canonical_finish::extend_step_names_for_generated;
