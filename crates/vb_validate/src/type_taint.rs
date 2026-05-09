#![forbid(unsafe_code)]
//! Type and taint validation for workflow documents.
//!
//! Tracks input/action/result types through workflow steps, enforces secret
//! taint propagation rules (rejecting SECRET_RESULT_LEAK), and validates
//! resource contract bounds against protocol hard limits.

use crate::{ValidationError, ValidationResult};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public value types
// ---------------------------------------------------------------------------

/// Supported value types for type checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// Null type.
    Null,
    /// Boolean type.
    Boolean,
    /// Numeric type (integer or float).
    Number,
    /// Text/string type.
    Text,
    /// Object type.
    Object,
    /// List type.
    List,
    /// Any type (type checking passes for all operations).
    Any,
}

impl ValueType {
    /// Returns the stable type name for diagnostics.
    pub const fn as_str(self) -> &'static str {
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

/// Taint marker for secret propagation tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Taint {
    /// No secret-derived data.
    Clean,
    /// Contains or derives from secret data.
    Secret,
}

impl Taint {
    /// Merges two taint markers; secret taint propagates.
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Secret, _) | (_, Self::Secret) => Self::Secret,
            (Self::Clean, Self::Clean) => Self::Clean,
        }
    }
}

/// Combined type and taint fact for a value or slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueFact {
    /// Inferred value type.
    pub value_type: ValueType,
    /// Secret taint status.
    pub taint: Taint,
}

impl ValueFact {
    /// Creates a clean fact with the given type.
    pub const fn clean(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Clean,
        }
    }

    /// Creates a secret-tainted fact with the given type.
    pub const fn secret(value_type: ValueType) -> Self {
        Self {
            value_type,
            taint: Taint::Secret,
        }
    }
}

// ---------------------------------------------------------------------------
// Workflow model
// ---------------------------------------------------------------------------

/// Input schema type declaration.
#[derive(Debug, Clone)]
pub struct InputDecl {
    /// Input name.
    pub name: String,
    /// Declared type.
    pub schema_type: ValueType,
    /// Whether this input is a secret.
    pub is_secret: bool,
}

/// Resource contract limits for validation.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum step count.
    pub max_steps: usize,
    /// Maximum slot count.
    pub max_slots: usize,
    /// Maximum constant pool size.
    pub max_constants: usize,
    /// Maximum accessor table entries.
    pub max_accessors: usize,
    /// Maximum expression programs.
    pub max_expressions: usize,
    /// Maximum expression stack depth.
    pub max_expr_stack: usize,
    /// Maximum deterministic step budget per scheduler tick.
    pub max_step_budget_per_tick: usize,
    /// Maximum input payload bytes.
    pub max_input_bytes: usize,
    /// Maximum output payload bytes.
    pub max_output_bytes: usize,
    /// Maximum blob bytes.
    pub max_blob_bytes: usize,
    /// Maximum IPC payload bytes.
    pub max_ipc_payload_bytes: usize,
    /// Maximum retry attempts.
    pub max_retry_attempts: usize,
    /// Maximum fanout branch count.
    pub max_fanout: usize,
    /// Maximum collect item count.
    pub max_collect_items: usize,
    /// Maximum queue depth.
    pub max_queue_depth: usize,
    /// Maximum journal batch bytes.
    pub max_journal_batch_bytes: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_steps: 1_000,
            max_slots: 65_535,
            max_constants: 8_192,
            max_accessors: 8_192,
            max_expressions: 4_096,
            max_expr_stack: 64,
            max_step_budget_per_tick: 10_000,
            max_input_bytes: 1_048_576,
            max_output_bytes: 1_048_576,
            max_blob_bytes: 16_777_216,
            max_ipc_payload_bytes: 1_048_576,
            max_retry_attempts: 10,
            max_fanout: 256,
            max_collect_items: 1_000,
            max_queue_depth: 1_024,
            max_journal_batch_bytes: 1_048_576,
        }
    }
}

/// Workflow model for type/taint validation.
#[derive(Debug, Clone, Default)]
pub struct WorkflowTypes {
    /// Declared inputs with their schemas.
    pub inputs: Vec<InputDecl>,
    /// Declared vars with their types.
    pub vars: Vec<(String, ValueType)>,
    /// Declared secret names.
    pub secrets: Vec<String>,
    /// Steps in declaration order.
    pub steps: Vec<StepTypes>,
    /// Resource contract limits.
    pub resource_contract: ResourceLimits,
}

/// Step model for type/taint validation.
#[derive(Debug, Clone)]
pub struct StepTypes {
    /// Step ID for diagnostics.
    pub id: String,
    /// Step kind.
    pub kind: StepKind,
}

/// Step behavior for type/taint checking.
#[derive(Debug, Clone)]
pub enum StepKind {
    /// Save step: writes a value into the step's slot.
    Save {
        /// Value being saved.
        value: TypedValue,
    },
    /// Choose step: branch on a boolean condition.
    Choose {
        /// Condition expression.
        condition: TypedValue,
    },
    /// Finish step: produces the workflow result.
    Finish {
        /// Result expression.
        result: TypedValue,
    },
}

/// Typed value for validation.
#[derive(Debug, Clone)]
pub enum TypedValue {
    /// A literal with known type.
    Literal(ValueType),
    /// A reference to a declared name (e.g., `$input.user`).
    Reference(String),
    /// A slot reference by step index.
    Slot(usize),
    /// A composite value (list/object) with sub-values.
    Composite(Vec<TypedValue>),
}

// ---------------------------------------------------------------------------
// Public validators
// ---------------------------------------------------------------------------

/// Validates types and taint for an entire workflow.
pub fn validate_types(workflow: &WorkflowTypes) -> ValidationResult<()> {
    let facts = Facts::build(workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    validate_step_types(workflow, &facts, &mut slots)
}

/// Validates secret taint tracking; rejects secret data leaking into results.
pub fn validate_taint(workflow: &WorkflowTypes) -> ValidationResult<()> {
    let facts = Facts::build(workflow);
    let mut slots = vec![None::<ValueFact>; workflow.steps.len()];
    validate_step_taint(workflow, &facts, &mut slots)
}

/// Validates resource contract bounds against protocol hard limits.
pub fn validate_resource_limits(
    workflow: &WorkflowTypes,
    hard_limits: &ResourceLimits,
) -> ValidationResult<()> {
    check_resource_bound(
        "max_steps",
        workflow.steps.len(),
        workflow.resource_contract.max_steps,
        hard_limits.max_steps,
    )?;
    check_resource_bound(
        "max_slots",
        workflow.steps.len(),
        workflow.resource_contract.max_slots,
        hard_limits.max_slots,
    )?;
    check_resource_bound(
        "max_constants",
        0,
        workflow.resource_contract.max_constants,
        hard_limits.max_constants,
    )?;
    check_declared_bound(
        "max_accessors",
        workflow.resource_contract.max_accessors,
        hard_limits.max_accessors,
    )?;
    check_declared_bound(
        "max_expressions",
        workflow.resource_contract.max_expressions,
        hard_limits.max_expressions,
    )?;
    check_declared_bound(
        "max_expr_stack",
        workflow.resource_contract.max_expr_stack,
        hard_limits.max_expr_stack,
    )?;
    check_declared_bound(
        "max_step_budget_per_tick",
        workflow.resource_contract.max_step_budget_per_tick,
        hard_limits.max_step_budget_per_tick,
    )?;
    check_declared_bound(
        "max_input_bytes",
        workflow.resource_contract.max_input_bytes,
        hard_limits.max_input_bytes,
    )?;
    check_declared_bound(
        "max_output_bytes",
        workflow.resource_contract.max_output_bytes,
        hard_limits.max_output_bytes,
    )?;
    check_declared_bound(
        "max_blob_bytes",
        workflow.resource_contract.max_blob_bytes,
        hard_limits.max_blob_bytes,
    )?;
    check_declared_bound(
        "max_ipc_payload_bytes",
        workflow.resource_contract.max_ipc_payload_bytes,
        hard_limits.max_ipc_payload_bytes,
    )?;
    check_declared_bound(
        "max_retry_attempts",
        workflow.resource_contract.max_retry_attempts,
        hard_limits.max_retry_attempts,
    )?;
    check_declared_bound(
        "max_fanout",
        workflow.resource_contract.max_fanout,
        hard_limits.max_fanout,
    )?;
    check_declared_bound(
        "max_collect_items",
        workflow.resource_contract.max_collect_items,
        hard_limits.max_collect_items,
    )?;
    check_declared_bound(
        "max_queue_depth",
        workflow.resource_contract.max_queue_depth,
        hard_limits.max_queue_depth,
    )?;
    check_declared_bound(
        "max_journal_batch_bytes",
        workflow.resource_contract.max_journal_batch_bytes,
        hard_limits.max_journal_batch_bytes,
    )
}

// ---------------------------------------------------------------------------
// Internal: resource checking
// ---------------------------------------------------------------------------

fn check_resource_bound(
    resource: &str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> ValidationResult<()> {
    check_declared_bound(resource, declared, hard_limit)?;
    if actual > declared {
        return Err(ValidationError::LimitExceeded {
            resource: resource.to_owned(),
        });
    }
    Ok(())
}

fn check_declared_bound(
    resource: &str,
    declared: usize,
    hard_limit: usize,
) -> ValidationResult<()> {
    if declared == 0 {
        return Err(ValidationError::LimitRequired {
            resource: resource.to_owned(),
        });
    }
    if declared > hard_limit {
        return Err(ValidationError::LimitExceeded {
            resource: resource.to_owned(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal: fact table
// ---------------------------------------------------------------------------

struct Facts {
    inputs: HashMap<String, ValueFact>,
    vars: HashMap<String, ValueFact>,
    secrets: HashMap<String, ValueFact>,
}

impl Facts {
    fn build(workflow: &WorkflowTypes) -> Self {
        Self {
            inputs: input_facts(&workflow.inputs),
            vars: var_facts(&workflow.vars),
            secrets: secret_facts(&workflow.secrets),
        }
    }

    fn resolve_reference(&self, reference: &str) -> ValueFact {
        let Some(body) = reference.strip_prefix('$') else {
            return ValueFact::clean(ValueType::Text);
        };
        let Some((root, tail)) = body.split_once('.') else {
            return ValueFact::clean(ValueType::Any);
        };
        let name = reference_name(tail);
        let fact = match root {
            "input" => self.inputs.get(name).copied(),
            "var" | "vars" => self.vars.get(name).copied(),
            "secrets" => self.secrets.get(name).copied(),
            _ => None,
        };
        match fact {
            Some(value) => value,
            None => ValueFact::clean(ValueType::Any),
        }
    }
}

fn input_facts(inputs: &[InputDecl]) -> HashMap<String, ValueFact> {
    let mut facts = HashMap::with_capacity(inputs.len());
    for input in inputs {
        let taint = if input.is_secret {
            Taint::Secret
        } else {
            Taint::Clean
        };
        match facts.entry(input.name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(ValueFact {
                    value_type: input.schema_type,
                    taint,
                });
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ValueFact {
                    value_type: input.schema_type,
                    taint,
                });
            }
        }
    }
    facts
}

fn var_facts(vars: &[(String, ValueType)]) -> HashMap<String, ValueFact> {
    let mut facts = HashMap::with_capacity(vars.len());
    for (name, vt) in vars {
        match facts.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(ValueFact::clean(*vt));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ValueFact::clean(*vt));
            }
        }
    }
    facts
}

fn secret_facts(secrets: &[String]) -> HashMap<String, ValueFact> {
    let mut facts = HashMap::with_capacity(secrets.len());
    for name in secrets {
        match facts.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.insert(ValueFact::secret(ValueType::Any));
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ValueFact::secret(ValueType::Any));
            }
        }
    }
    facts
}

fn reference_name(tail: &str) -> &str {
    match tail.split_once('.') {
        Some((name, _)) => name,
        None => tail,
    }
}

// ---------------------------------------------------------------------------
// Internal: step validation
// ---------------------------------------------------------------------------

fn validate_step_types(
    workflow: &WorkflowTypes,
    facts: &Facts,
    slots: &mut [Option<ValueFact>],
) -> ValidationResult<()> {
    for (index, step) in workflow.steps.iter().enumerate() {
        match &step.kind {
            StepKind::Save { value } => {
                let fact = resolve_value(value, facts, slots);
                write_slot(slots, index, fact);
            }
            StepKind::Choose { condition } => {
                let fact = resolve_value(condition, facts, slots);
                require_boolean(fact.value_type)?;
            }
            StepKind::Finish { .. } => {}
        }
    }
    Ok(())
}

fn validate_step_taint(
    workflow: &WorkflowTypes,
    facts: &Facts,
    slots: &mut [Option<ValueFact>],
) -> ValidationResult<()> {
    for (index, step) in workflow.steps.iter().enumerate() {
        match &step.kind {
            StepKind::Save { value } => {
                let fact = resolve_value(value, facts, slots);
                write_slot(slots, index, fact);
            }
            StepKind::Choose { .. } => {
                // Taint pass only: no taint is produced or leaked by a branch
                // condition. Type checking of the condition is handled by
                // validate_step_types.
            }
            StepKind::Finish { result } => {
                let fact = resolve_value(result, facts, slots);
                if fact.taint == Taint::Secret {
                    return Err(ValidationError::SecretResultLeak);
                }
            }
        }
    }
    Ok(())
}

fn write_slot(slots: &mut [Option<ValueFact>], index: usize, fact: ValueFact) {
    if let Some(slot) = slots.get_mut(index) {
        *slot = Some(fact);
    }
}

fn resolve_value(value: &TypedValue, facts: &Facts, slots: &[Option<ValueFact>]) -> ValueFact {
    match value {
        TypedValue::Literal(vt) => ValueFact::clean(*vt),
        TypedValue::Reference(name) => facts.resolve_reference(name),
        TypedValue::Slot(index) => match slots.get(*index).and_then(|s| *s) {
            Some(value) => value,
            None => ValueFact::clean(ValueType::Any),
        },
        TypedValue::Composite(values) => resolve_composite(values, facts, slots),
    }
}

fn resolve_composite(
    values: &[TypedValue],
    facts: &Facts,
    slots: &[Option<ValueFact>],
) -> ValueFact {
    let mut taint = Taint::Clean;
    for value in values {
        let fact = resolve_value(value, facts, slots);
        taint = taint.merge(fact.taint);
    }
    ValueFact {
        value_type: ValueType::Any,
        taint,
    }
}

fn require_boolean(actual: ValueType) -> ValidationResult<()> {
    if matches!(actual, ValueType::Boolean | ValueType::Any) {
        Ok(())
    } else {
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: actual.as_str().to_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "type_taint_tests.rs"]
mod tests;
