impl<'a> Facts<'a> {
    fn new(ast: &'a WorkflowAst) -> Self {
        Self {
            inputs: input_facts(&ast.inputs),
            vars: value_facts(&ast.vars),
            secrets: secret_facts(&ast.secrets),
            slots: vec![None; ast.steps.len()],
        }
    }

    fn write_slot(&mut self, index: usize, fact: ValueFact) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Some(fact);
        }
    }

    fn read_slot(&self, index: usize, field: &'static str) -> Result<ValueFact, CompileError> {
        match self.slots.get(index).and_then(|slot| *slot) {
            Some(fact) => Ok(fact),
            None => Err(CompileError::UnknownSlotType { field, slot: index }),
        }
    }
}

fn input_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        match facts.entry(entry.name.as_ref()) {
            std::collections::hash_map::Entry::Occupied(mut fact) => {
                fact.insert(input_schema_fact(&entry.value));
            }
            std::collections::hash_map::Entry::Vacant(fact) => {
                fact.insert(input_schema_fact(&entry.value));
            }
        }
    }
    facts
}

fn input_schema_fact(value: &AstValue) -> ValueFact {
    match value {
        AstValue::Text(name) => ValueFact::clean(schema_type(name)),
        AstValue::Mapping(entries) => schema_mapping_fact(entries),
        _ => ValueFact::clean(ValueType::Any),
    }
}

fn schema_mapping_fact(entries: &[AstMapEntry<AstValue>]) -> ValueFact {
    let mut value_type = ValueType::Any;
    let mut taint = Taint::Clean;
    for entry in entries {
        match (entry.name.as_ref(), &entry.value) {
            ("is", AstValue::Text(name)) => value_type = schema_type(name),
            ("secret", AstValue::Bool(true)) => taint = Taint::Secret,
            _ => {}
        }
    }
    ValueFact { value_type, taint }
}

fn schema_type(name: &str) -> ValueType {
    match name {
        "text" => ValueType::Text,
        "number" => ValueType::Number,
        "boolean" => ValueType::Boolean,
        "object" => ValueType::Object,
        "list" | "list<any>" | "list<text>" | "list<number>" | "list<boolean>" => ValueType::List,
        _ => ValueType::Any,
    }
}

fn value_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        match facts.entry(entry.name.as_ref()) {
            std::collections::hash_map::Entry::Occupied(mut fact) => {
                fact.insert(value_fact(&entry.value, None));
            }
            std::collections::hash_map::Entry::Vacant(fact) => {
                fact.insert(value_fact(&entry.value, None));
            }
        }
    }
    facts
}

fn secret_facts<T>(entries: &[AstMapEntry<T>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        match facts.entry(entry.name.as_ref()) {
            std::collections::hash_map::Entry::Occupied(mut fact) => {
                fact.insert(ValueFact {
                    value_type: ValueType::Any,
                    taint: Taint::Secret,
                });
            }
            std::collections::hash_map::Entry::Vacant(fact) => {
                fact.insert(ValueFact {
                    value_type: ValueType::Any,
                    taint: Taint::Secret,
                });
            }
        }
    }
    facts
}

fn validate_steps(ast: &WorkflowAst, facts: &mut Facts<'_>) -> Result<(), CompileErrors> {
    let mut errors = Vec::new();
    for (index, step) in ast.steps.iter().enumerate() {
        match &step.kind {
            StepKindAst::Run { input, .. } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "run.input") {
                    errors.push(e);
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Save { fields } => facts.write_slot(index, save_fact(fields, facts)),
            StepKindAst::Choose { condition, .. } => {
                if let Err(e) = validate_condition(condition, facts) {
                    errors.push(e);
                }
            }
            StepKindAst::ForEach { input, item, .. } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "for_each.input") {
                    errors.push(e);
                }
                facts.write_slot(item.as_usize(), ValueFact::clean(ValueType::Any));
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Together { .. } => {
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Collect { source, .. } => {
                if let Err(e) = facts.read_slot(source.as_usize(), "collect.source") {
                    errors.push(e);
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Reduce {
                input, accumulator, ..
            } => {
                if let Err(e) = facts.read_slot(input.as_usize(), "reduce.input") {
                    errors.push(e);
                }
                facts.write_slot(accumulator.as_usize(), ValueFact::clean(ValueType::Any));
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Repeat { .. } => {
                if let Some(attempt_slot) = index.checked_add(1) {
                    facts.write_slot(attempt_slot, ValueFact::clean(ValueType::Any));
                } else {
                    errors.push(CompileError::SlotIndexOutOfRange { value: i64::MAX });
                }
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Wait { .. } => facts.write_slot(index, ValueFact::clean(ValueType::Any)),
            StepKindAst::Ask { answer, .. } => {
                facts.write_slot(answer.as_usize(), ValueFact::clean(ValueType::Any));
                facts.write_slot(index, ValueFact::clean(ValueType::Any));
            }
            StepKindAst::Finish { result } => {
                if let Err(e) = validate_public_result(result, facts) {
                    errors.push(e);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(CompileErrors(errors))
    }
}

fn save_fact(fields: &[AstMapEntry<AstValue>], facts: &Facts<'_>) -> ValueFact {
    match single_value_field(fields) {
        Some(value) => value_fact(value, Some(facts)),
        None => optional_object_fact(fields, Some(facts)),
    }
}

fn single_value_field(fields: &[AstMapEntry<AstValue>]) -> Option<&AstValue> {
    match fields {
        [entry] if entry.name.as_ref() == "value" => Some(&entry.value),
        _ => None,
    }
}

fn validate_condition(expression: &AstExpression, facts: &Facts<'_>) -> Result<(), CompileError> {
    let fact = expression_fact(expression, facts, "choose.condition")?;
    if matches!(fact.value_type, ValueType::Boolean | ValueType::Any) {
        Ok(())
    } else {
        Err(CompileError::TypeMismatch {
            field: "choose.condition",
            expected: "boolean",
            found: fact.value_type.as_str(),
        })
    }
}

fn validate_public_result(
    expression: &AstExpression,
    facts: &Facts<'_>,
) -> Result<(), CompileError> {
    let fact = expression_fact(expression, facts, "finish.result")?;
    if fact.taint == Taint::Secret {
        Err(CompileError::SecretTaintLeak {
            field: "finish.result",
        })
    } else {
        Ok(())
    }
}

fn expression_fact(
    expression: &AstExpression,
    facts: &Facts<'_>,
    field: &'static str,

mod impl2;
