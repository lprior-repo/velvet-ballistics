// Frozen pre-extraction generic body used only for differential characterization.

fn reference_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
) -> Result<Option<(SlotIdx, ExprType)>, CompileErrors> {
    let mut local = BranchBindings::new(scope, primitive == "for_each");
    let layout = canonical_layout(body, start.as_usize()).map_err(|e| CompileErrors(vec![e]))?;
    let mut names = vec![Box::from(""); start.as_usize()];
    names.extend(canonical_step_names(body, &layout).map_err(|e| CompileErrors(vec![e]))?);
    let mut result = None;
    let mut offset = 0usize;
    for (index, step) in body.iter().enumerate() {
        reject_body_controls(step, index)?;
        let id = body_id(start, offset, primitive)?;
        offset = offset
            .checked_add(
                canonical_step_width_with_condition(step).map_err(|e| CompileErrors(vec![e]))?,
            )
            .ok_or_else(|| index_error(offset))?;
        let next_index = index.checked_add(1).ok_or_else(|| index_error(index))?;
        let next = if next_index == body.len() {
            common_next
        } else {
            body_id(start, offset, primitive)?
        };
        let control = crate::mod_compile_lowering::part_02::canonical_controls::prepare(
            step,
            id,
            Some(next),
            &names,
            local.view(),
            builder,
        )?;
        let success = control.next.ok_or_else(|| index_error(id.as_usize()))?;
        if let Some(slot) = local.lower(&control.step, control.body, success, &names, builder)? {
            result = Some((slot, local.type_of(slot).unwrap_or(ExprType::Unknown)));
        }
        crate::mod_compile_lowering::part_02::canonical_controls::complete(&control, builder);
    }
    Ok(result)
}
