//! Leaf calls preserve ancestor reads before publishing the local binding.

use super::*;
use vb_core::ActionId;

impl BranchBindings<'_> {
    pub(super) fn lower_set(
        &mut self,
        output: &str,
        value: &str,
        target: BodyTarget,
        builder: &mut SlotCompiler,
    ) -> Result<(), CompileErrors> {
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
        Ok(())
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
        let input = resolve_slot_reference_scoped(input, self.view())
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
