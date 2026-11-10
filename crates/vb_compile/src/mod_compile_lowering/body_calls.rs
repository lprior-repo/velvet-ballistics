//! Leaf and nested-container call implementations for body lowering.
//!
//! Companion `impl` block for [`BranchBindings`]: leaves bind value slots
//! here, and nested `repeat`/`choose` containers delegate to their top-level
//! lowering entry points.

use super::BranchBindings;
use super::*;
use crate::mod_compile_lowering::part_04::{RepeatLowering, lower_canonical_repeat};
use crate::mod_compile_lowering::part_14::{ChooseScope, lower_canonical_choose_in_scope};
use vb_core::ActionId;

impl<'a> BranchBindings<'a> {
    pub(super) fn lower_nested_repeat(
        &mut self,
        max_attempts: u16,
        body: &[crate::StepAst],
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<SlotIdx, CompileErrors> {
        lower_canonical_repeat(
            self.ctx.index,
            id,
            RepeatLowering {
                max_attempts,
                body,
                next: Some(next),
                scope: self.view(),
                inputs: self.ctx.inputs,
                step_names: self.ctx.step_names,
            },
            builder,
        )
    }

    pub(super) fn lower_nested_choose(
        &mut self,
        branches: &[crate::ChooseBranch],
        otherwise: Option<&str>,
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<(), CompileErrors> {
        lower_canonical_choose_in_scope(
            id,
            branches,
            otherwise,
            Some(next),
            ChooseScope {
                index: self.ctx.index,
                names: self.ctx.step_names,
                bindings: self.view(),
                inputs: self.ctx.inputs,
            },
            builder,
        )
    }

    pub(super) fn lower_set(
        &mut self,
        output: &str,
        value: &str,
        target: BodyTarget,
        builder: &mut SlotCompiler,
    ) -> Result<ExprType, CompileErrors> {
        validate_output_alias(output).map_err(|e| CompileErrors(vec![e]))?;
        if !self.local_outputs.insert(output.to_owned()) {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: output.into(),
            }]));
        }
        if self.strict_unknown {
            validate_for_each_expression(value, self.view())?;
        }
        let inferred = lower_canonical_set_expression(
            target.id,
            target.slot,
            value,
            Some(target.next),
            self.view(),
            builder,
        )?;
        self.outputs.insert(output.to_owned(), target.slot);
        self.types.insert(target.slot, inferred);
        Ok(inferred)
    }

    pub(super) fn lower_do(
        &self,
        action: &str,
        input: &str,
        target: BodyTarget,
        builder: &mut SlotCompiler,
    ) -> Result<(), CompileErrors> {
        let action = action.parse::<u16>().map_err(|_| {
            CompileErrors(vec![CompileError::StepFieldShape {
                step: target.id.as_usize(),
                field: "action",
                expected: "numeric action ID (u16)",
            }])
        })?;
        let input = resolve_slot_reference(input, &self.outputs, &self.steps)
            .map_err(|e| CompileErrors(vec![e]))?;
        let node = lower_do(
            target.id,
            ActionId::new(action),
            input,
            Some(target.slot),
            Some(target.next),
            builder,
        );
        builder.push_node(node);
        Ok(())
    }
}
