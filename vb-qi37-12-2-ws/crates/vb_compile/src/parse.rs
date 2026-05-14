#![forbid(unsafe_code)]
fn build_workflow_parts(text: &str, doc: &Yaml<'_>) -> Result<WorkflowParts, CompileError> {
    validate_workflow_document_shape(doc)?;

    let name = required_string_field(doc, "name")?;
    let steps = required_sequence_field(doc, "steps")?;
    let digest = WorkflowDigest::from_bytes(blake3::hash(text.as_bytes()).into());
    let mut builder = WorkflowBuilder::new();
    let last_step = steps.len().checked_sub(1).ok_or(CompileError::EmptySteps)?;
    let source_ir_starts = build_source_ir_starts(steps)?;

    for (index, step) in steps.iter().enumerate() {
        let id = source_ir_start(&source_ir_starts, index)?;
        let next = optional_source_ir_start(&source_ir_starts, index)?;
        let nodes = compile_step(
            step,
            index,
            last_step,
            id,
            next,
            &source_ir_starts,
            &mut builder,
        )?;
        builder.nodes.extend(nodes);
    }
    Ok(WorkflowParts {
        name: Box::<str>::from(name),
        digest,
        slot_count: builder.slot_count()?,
        symbols_count: 0,
        nodes: builder.nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: builder.constants.into_boxed_slice(),
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
    })
}

fn build_source_ir_starts(steps: &saphyr::Sequence<'_>) -> Result<Vec<StepIdx>, CompileError> {
    let mut starts = Vec::with_capacity(steps.len());
    let mut cursor = 0usize;
    for (index, step) in steps.iter().enumerate() {
        starts.push(step_idx(cursor)?);
        cursor = cursor
            .checked_add(compiled_step_width(step, index)?)
            .ok_or(CompileError::StepIndexOutOfRange { value: cursor })?;
    }
    Ok(starts)
}

fn compiled_step_width(step: &Yaml<'_>, index: usize) -> Result<usize, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    match primitive {
        StepPrimitive::Ask | StepPrimitive::ForEach | StepPrimitive::Together => Ok(2),
        StepPrimitive::Collect | StepPrimitive::Reduce | StepPrimitive::Repeat => Ok(3),
        StepPrimitive::Finish => {
            let result = required_step_field(body, index, "result")?;
            if finish_result_slot(result, index)?.is_some() {
                Ok(1)
            } else {
                Ok(2)
            }
        }
        _ => Ok(1),
    }
}

fn source_ir_start(starts: &[StepIdx], index: usize) -> Result<StepIdx, CompileError> {
    starts
        .get(index)
        .copied()
        .ok_or(CompileError::StepIndexOutOfRange { value: index })
}

fn optional_source_ir_start(
    starts: &[StepIdx],
    index: usize,
) -> Result<Option<StepIdx>, CompileError> {
    let next = index
        .checked_add(1)
        .ok_or(CompileError::StepIndexOutOfRange { value: index })?;
    Ok(starts.get(next).copied())
}
