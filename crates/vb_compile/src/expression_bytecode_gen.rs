impl ExpressionReferenceResolver for SlotAccessorReferenceResolver<'_> {
    fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError> {
        let lowered = lower_slot_reference(reference, self.accessors)?;
        Ok(lowered)
    }
}

fn lower_slot_reference(
    reference: &str,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let (root, tail) = parse_slot_reference_parts(reference)?;
    let (slot, path) = split_reference_tail(tail);
    let root_slot = parse_slot_reference_index(reference, slot)?;
    match path {
        Some(path) => lower_accessor_reference(reference, root, slot, path, root_slot, accessors),
        None => Ok(ExprOp::LoadSlot(root_slot)),
    }
}

fn parse_slot_reference_parts(reference: &str) -> Result<(&str, &str), CompileError> {
    let Some(body) = reference.strip_prefix('$') else {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(reference),
        });
    };
    let Some((root, tail)) = body.split_once('.') else {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(body),
        });
    };
    if !matches!(root, "slot" | "slots") {
        return Err(CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from(reference),
            root: Box::<str>::from(root),
        });
    }
    Ok((root, tail))
}

fn split_reference_tail(tail: &str) -> (&str, Option<&str>) {
    match tail.split_once('.') {
        Some((slot, path)) => (slot, Some(path)),
        None => (tail, None),
    }
}

fn parse_slot_reference_index(reference: &str, slot: &str) -> Result<SlotIdx, CompileError> {
    let parsed = slot
        .parse::<u16>()
        .map_err(|_| CompileError::UnknownReferenceName {
            kind: "slot",
            reference: Box::<str>::from(reference),
            name: Box::<str>::from(slot),
        })?;
    Ok(SlotIdx::new(parsed))
}

fn lower_accessor_reference(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
    root_slot: SlotIdx,
    accessors: &mut Vec<AccessorProgram>,
) -> Result<ExprOp, CompileError> {
    let path = numeric_path_segments(reference, root, slot, path)?;
    let index = u16::try_from(accessors.len()).map_err(|_| {
        CompileError::ExpressionLoweringUnsupported {
            feature: "accessor table overflow".into(),
        }
    })?;
    accessors.push(AccessorProgram {
        root: root_slot,
        path: path.into_boxed_slice(),
    });
    Ok(ExprOp::LoadAccessor(AccessorIdx::new(index)))
}

fn numeric_path_segments(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
) -> Result<Vec<PathSegment>, CompileError> {
    let mut segments = Vec::new();
    for segment in path.split('.') {
        let index = parse_list_index_segment(reference, root, slot, path, segment)?;
        segments.push(PathSegment::Index(index));
    }
    Ok(segments)
}

fn parse_list_index_segment(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
    segment: &str,
) -> Result<u32, CompileError> {
    segment
        .parse::<u32>()
        .map_err(|_| unsupported_accessor_reference(reference, root, slot, path))
}

fn unsupported_accessor_reference(
    reference: &str,
    root: &str,
    slot: &str,
    path: &str,
) -> CompileError {
    CompileError::UnsupportedAccessorReference {
        reference: Box::<str>::from(reference),
        root: Box::<str>::from(format!("{root}.{slot}")),
        path: Box::<str>::from(path),
    }
}

fn lower_expr(
    expression: &ParsedExpression,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    match expression {
        ParsedExpression::Literal(literal) => lower_literal(literal, constants, ops),
        ParsedExpression::Unary { op, expr } => lower_unary(*op, expr, constants, ops, resolver),
        ParsedExpression::Binary { op, left, right } => {
            lower_binary(*op, left, right, constants, ops, resolver)
        }
        ParsedExpression::HelperCall { name, args } => {
            lower_helper(*name, args, constants, ops, resolver)
        }
        ParsedExpression::Reference(reference) => lower_reference(reference, ops, resolver),
    }
}

fn lower_reference(
    reference: &str,
    ops: &mut Vec<ExprOp>,
    resolver: &mut impl ExpressionReferenceResolver,
) -> Result<(), CompileError> {
    ops.push(resolver.resolve_reference(reference)?);
    Ok(())
}

fn lower_literal(
    literal: &ExpressionLiteral,
    constants: &mut Vec<ConstValue>,
    ops: &mut Vec<ExprOp>,
) -> Result<(), CompileError> {
    let value = match literal {
        ExpressionLiteral::Null => ConstValue::Null,
        ExpressionLiteral::Bool(value) => ConstValue::Bool(*value),
        ExpressionLiteral::I64(value) => ConstValue::I64(*value),
        ExpressionLiteral::Text(_) => {
            return Err(CompileError::ExpressionLoweringUnsupported {
                feature: "text constants".into(),

mod gen2;
