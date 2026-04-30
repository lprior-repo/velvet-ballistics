use crate::CompileError;
use crate::ast::{AstExpression, AstMapEntry, AstValue, StepKindAst, WorkflowAst};
use std::collections::HashMap;

pub(crate) fn validate_workflow_ast(ast: &WorkflowAst) -> Result<(), CompileError> {
    let mut facts = Facts::new(ast);
    validate_steps(ast, &mut facts)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Null,
    Boolean,
    Number,
    Text,
    Object,
    List,
    Any,
}

impl ValueType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Text => "text",
            Self::Object => "object",
            Self::List => "list",
            Self::Any => "any",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Taint {
    Clean,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValueFact {
    value_type: ValueType,
    taint: Taint,
}

impl ValueFact {
    const fn clean(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Clean,
        }
    }

    const fn merge(self, other: Self) -> Self {
        let taint = match (self.taint, other.taint) {
            (Taint::Secret, _) | (_, Taint::Secret) => Taint::Secret,
            (Taint::Clean, Taint::Clean) => Taint::Clean,
        };
        Self {
            value_type: self.value_type,
            taint,
        }
    }
}

struct Facts<'a> {
    inputs: HashMap<&'a str, ValueFact>,
    vars: HashMap<&'a str, ValueFact>,
    secrets: HashMap<&'a str, ValueFact>,
    slots: Vec<Option<ValueFact>>,
}

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
        let _ = facts.insert(entry.name.as_ref(), input_schema_fact(&entry.value));
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
        "any" => ValueType::Any,
        _ => ValueType::Any,
    }
}

fn value_facts(entries: &[AstMapEntry<AstValue>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        let _ = facts.insert(entry.name.as_ref(), value_fact(&entry.value, None));
    }
    facts
}

fn secret_facts<T>(entries: &[AstMapEntry<T>]) -> HashMap<&str, ValueFact> {
    let mut facts = HashMap::with_capacity(entries.len());
    for entry in entries {
        let _ = facts.insert(
            entry.name.as_ref(),
            ValueFact {
                value_type: ValueType::Any,
                taint: Taint::Secret,
            },
        );
    }
    facts
}

fn validate_steps(ast: &WorkflowAst, facts: &mut Facts<'_>) -> Result<(), CompileError> {
    for (index, step) in ast.steps.iter().enumerate() {
        match &step.kind {
            StepKindAst::Save { fields } => facts.write_slot(index, save_fact(fields, facts)),
            StepKindAst::Choose { condition, .. } => validate_condition(condition, facts)?,
            StepKindAst::Finish { result } => validate_public_result(result, facts)?,
        }
    }
    Ok(())
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
) -> Result<ValueFact, CompileError> {
    match expression {
        AstExpression::Slot(slot) => facts.read_slot(slot.as_usize(), field),
        AstExpression::Reference(reference) => Ok(reference_fact(reference, Some(facts))),
        AstExpression::Literal(value) => Ok(value_fact(value, Some(facts))),
    }
}

fn value_fact(value: &AstValue, facts: Option<&Facts<'_>>) -> ValueFact {
    match value {
        AstValue::Null => ValueFact::clean(ValueType::Null),
        AstValue::Bool(_) => ValueFact::clean(ValueType::Boolean),
        AstValue::I64(_) => ValueFact::clean(ValueType::Number),
        AstValue::Text(_) => ValueFact::clean(ValueType::Text),
        AstValue::Reference(reference) => reference_fact(reference, facts),
        AstValue::Sequence(values) => sequence_fact(values, facts),
        AstValue::Mapping(entries) => optional_object_fact(entries, facts),
    }
}

fn sequence_fact(values: &[AstValue], facts: Option<&Facts<'_>>) -> ValueFact {
    let mut fact = ValueFact::clean(ValueType::List);
    for value in values {
        fact = fact.merge(value_fact(value, facts));
    }
    fact
}

fn optional_object_fact(entries: &[AstMapEntry<AstValue>], facts: Option<&Facts<'_>>) -> ValueFact {
    let mut fact = ValueFact::clean(ValueType::Object);
    for entry in entries {
        fact = fact.merge(value_fact(&entry.value, facts));
    }
    fact
}

fn reference_fact(reference: &str, facts: Option<&Facts<'_>>) -> ValueFact {
    let Some(facts) = facts else {
        return ValueFact::clean(ValueType::Any);
    };
    let Some(body) = reference.strip_prefix('$') else {
        return ValueFact::clean(ValueType::Text);
    };
    let Some((root, tail)) = body.split_once('.') else {
        return ValueFact::clean(ValueType::Any);
    };
    let name = match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    };
    let table = match root {
        "input" => Some(&facts.inputs),
        "var" | "vars" => Some(&facts.vars),
        "secret" | "secrets" => Some(&facts.secrets),
        _ => None,
    };
    match table.and_then(|values| values.get(name)) {
        Some(fact) => *fact,
        None => ValueFact::clean(ValueType::Any),
    }
}

#[cfg(test)]
mod tests;
