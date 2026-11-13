use vb_compile::CompileError;
use vb_core::{StepIdx, WorkflowError};

pub(super) fn assert_nested_together_rejected(
    source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let errors = match vb_compile::compile_workflow(source.as_bytes()) {
        Ok(_) => return Err("nested Together source unexpectedly compiled".into()),
        Err(errors) => errors,
    };
    let exact = errors.0.iter().any(|error| {
        matches!(
            error,
            CompileError::Workflow(WorkflowError::NestedTogether { outer, inner })
                if *outer == StepIdx::new(2) && *inner == StepIdx::new(4)
        )
    });
    if exact {
        Ok(())
    } else {
        Err(format!("expected exact NestedTogether {{ outer: StepIdx(2), inner: StepIdx(4) }}, got {errors:?}").into())
    }
}
