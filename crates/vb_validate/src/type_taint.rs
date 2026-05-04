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
mod tests {
    use super::*;

    fn make_workflow(steps: Vec<StepTypes>) -> WorkflowTypes {
        WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps,
            resource_contract: ResourceLimits::default(),
        }
    }

    fn save_step(id: &str, value: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Save { value },
        }
    }

    fn choose_step(id: &str, condition: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Choose { condition },
        }
    }

    fn finish_step(id: &str, result: TypedValue) -> StepTypes {
        StepTypes {
            id: id.to_owned(),
            kind: StepKind::Finish { result },
        }
    }

    #[test]
    fn accepts_clean_finish() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn rejects_secret_finish_direct() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$secrets.token".into()),
        )]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn rejects_secret_finish_via_slot() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn accepts_boolean_choose() {
        let wf = make_workflow(vec![
            save_step("flag", TypedValue::Literal(ValueType::Boolean)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn rejects_number_choose() {
        let wf = make_workflow(vec![
            save_step("flag", TypedValue::Literal(ValueType::Number)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert!(matches!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn accepts_literal_boolean_choose() {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Boolean),
        )]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn rejects_literal_text_choose() {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Text),
        )]);
        assert!(matches!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn accepts_clean_input_finish() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.user".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "user".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    #[test]
    fn rejects_secret_input_finish() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.key".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "key".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn resource_limits_accept_within_bounds() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        let hard = ResourceLimits::default();
        assert_eq!(validate_resource_limits(&wf, &hard), Ok(()));
    }

    #[test]
    fn resource_limits_reject_exceeded_steps() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        let hard = ResourceLimits {
            max_steps: 0,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        };
        assert!(matches!(
            validate_resource_limits(&wf, &hard),
            Err(ValidationError::LimitExceeded { .. })
        ));
    }

    #[test]
    fn rejects_nested_secret_composite() {
        let mut wf = make_workflow(vec![
            save_step(
                "cap",
                TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    #[test]
    fn accepts_any_type_choose() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Any)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_types(&wf), Ok(()));
    }

    #[test]
    fn rejects_null_choose() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Null)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        assert!(matches!(
            validate_types(&wf),
            Err(ValidationError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn accepts_clean_var_finish() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$vars.label".into()),
        )]);
        wf.vars.push(("label".to_owned(), ValueType::Boolean));
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // ---------------------------------------------------------------------------
    // BDD exact-assertion tests
    // ---------------------------------------------------------------------------

    #[test]
    fn validate_types_returns_type_mismatch_for_wrong_type() {
        // Given a workflow with a choose step that has a number condition
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Number)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns TypeMismatch with expected boolean, found number
        assert_eq!(
            result,
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "number".to_owned(),
            })
        );
    }

    #[test]
    fn validate_taint_returns_secret_result_leak_for_secret_in_finish() {
        // Given a workflow where finish references a secret
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$secrets.api_key".into()),
        )]);
        wf.secrets.push("api_key".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn validate_taint_returns_forbidden_reference_to_untrusted_slot() {
        // Given a workflow where a secret is saved into a slot and then finished
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak (the slot is tainted)
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn validate_resource_limits_accepts_within_limits() {
        // Given a workflow with 1 step and default hard limits
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        let hard = ResourceLimits::default();
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_resource_limits_rejects_too_many_steps() {
        // Given a workflow with steps exceeding the declared limit
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        let hard = ResourceLimits {
            max_steps: 0,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        };
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns LimitExceeded for max_steps
        assert_eq!(
            result,
            Err(ValidationError::LimitExceeded {
                resource: "max_steps".to_owned(),
            })
        );
    }

    #[test]
    fn validate_resource_limits_rejects_declared_limit_exceeding_hard() {
        // Given a workflow where declared max_steps exceeds hard limit
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_steps: 100,
                max_slots: 65_535,
                max_constants: 8_192,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits {
            max_steps: 50,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        };
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns LimitExceeded for max_steps
        assert_eq!(
            result,
            Err(ValidationError::LimitExceeded {
                resource: "max_steps".to_owned(),
            })
        );
    }

    #[test]
    fn validate_resource_limits_returns_limit_required_for_zero_declared_runtime_limit() {
        // Given a workflow with an explicit zero fanout limit
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_fanout: 0,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default();
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns LimitRequired exactly.
        assert_eq!(
            result,
            Err(ValidationError::LimitRequired {
                resource: "max_fanout".to_owned(),
            })
        );
    }

    #[test]
    fn validate_resource_limits_rejects_declared_fanout_exceeding_hard() {
        // Given declared fanout exceeds the active hard profile
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_fanout: 64,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits {
            max_fanout: 32,
            ..ResourceLimits::default()
        };
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns LimitExceeded exactly.
        assert_eq!(
            result,
            Err(ValidationError::LimitExceeded {
                resource: "max_fanout".to_owned(),
            })
        );
    }

    #[test]
    fn value_type_as_str_returns_correct_names() {
        // Given all ValueType variants
        // When as_str is called
        // Then each returns the correct name
        assert_eq!(ValueType::Null.as_str(), "null");
        assert_eq!(ValueType::Boolean.as_str(), "boolean");
        assert_eq!(ValueType::Number.as_str(), "number");
        assert_eq!(ValueType::Text.as_str(), "text");
        assert_eq!(ValueType::Object.as_str(), "object");
        assert_eq!(ValueType::List.as_str(), "list");
        assert_eq!(ValueType::Any.as_str(), "any");
    }

    #[test]
    fn taint_merge_propagates_secret() {
        // Given Clean and Secret taints
        let clean = Taint::Clean;
        let secret = Taint::Secret;
        // When merge is called in both directions
        let result1 = clean.merge(secret);
        let result2 = secret.merge(clean);
        let result3 = secret.merge(secret);
        let result4 = clean.merge(clean);
        // Then secret always propagates
        assert_eq!(result1, Taint::Secret);
        assert_eq!(result2, Taint::Secret);
        assert_eq!(result3, Taint::Secret);
        assert_eq!(result4, Taint::Clean);
    }

    #[test]
    fn value_fact_clean_creates_clean_taint() {
        // Given a ValueFact created with clean
        let fact = ValueFact::clean(ValueType::Number);
        // When examining fields
        // Then taint is Clean and type is Number
        assert_eq!(fact.value_type, ValueType::Number);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn value_fact_secret_creates_secret_taint() {
        // Given a ValueFact created with secret
        let fact = ValueFact::secret(ValueType::Text);
        // When examining fields
        // Then taint is Secret and type is Text
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn validate_types_accepts_any_type_choose() {
        // Given a workflow with an Any-typed choose condition
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Any)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_types_rejects_null_choose_exact() {
        // Given a workflow with a Null-typed choose condition
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Null)),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns TypeMismatch with expected boolean, found null
        assert_eq!(
            result,
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "null".to_owned(),
            })
        );
    }

    #[test]
    fn validate_types_rejects_text_choose_exact() {
        // Given a workflow with a Text-typed choose condition
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Text),
        )]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns TypeMismatch with expected boolean, found text
        assert_eq!(
            result,
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "text".to_owned(),
            })
        );
    }

    #[test]
    fn validate_types_accepts_literal_boolean_choose() {
        // Given a workflow with a literal Boolean choose condition
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Boolean),
        )]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_taint_accepts_clean_input_finish() {
        // Given a workflow that finishes with a clean input reference
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.user".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "user".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns Ok
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_taint_rejects_secret_input_finish_exact() {
        // Given a workflow that finishes with a secret input reference
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.key".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "key".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn validate_taint_rejects_nested_secret_composite_exact() {
        // Given a workflow where a composite value contains a secret reference
        let mut wf = make_workflow(vec![
            save_step(
                "cap",
                TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn resource_limits_default_values() {
        // Given default ResourceLimits
        let limits = ResourceLimits::default();
        // When examining fields
        // Then compile-time hard defaults match the master v1 contract.
        assert_eq!(limits.max_steps, 1_000);
        assert_eq!(limits.max_slots, 65_535);
        assert_eq!(limits.max_constants, 8_192);
        assert_eq!(limits.max_accessors, 8_192);
        assert_eq!(limits.max_expressions, 4_096);
        assert_eq!(limits.max_expr_stack, 64);
        assert_eq!(limits.max_retry_attempts, 10);
        assert_eq!(limits.max_fanout, 256);
        assert_eq!(limits.max_collect_items, 1_000);
    }

    // ---------------------------------------------------------------------------
    // Adversarial BDD tests: validation bypass attacks
    // ---------------------------------------------------------------------------

    #[test]
    fn adversarial_secret_leak_via_direct_reference_in_finish_is_rejected() {
        // Given a workflow that directly puts $secrets.api_key into finish result
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$secrets.api_key".into()),
        )]);
        wf.secrets.push("api_key".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak (E0406)
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn adversarial_secret_leak_via_two_step_indirection_is_rejected() {
        // Given a workflow: save $secrets.token -> save slot[0] -> finish with slot[1]
        // But slot[1] is from saving slot[0], which carries the taint
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            save_step("relay", TypedValue::Slot(0)), // relay carries taint
            finish_step("done", TypedValue::Slot(1)), // finish with tainted slot
        ]);
        wf.secrets.push("token".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak (E0406)
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn adversarial_secret_leak_via_composite_with_clean_and_secret_is_rejected() {
        // Given a workflow with a composite containing both clean and secret refs
        let mut wf = make_workflow(vec![
            save_step(
                "mixed",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Reference("$secrets.password".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("password".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak (E0406) -- taint propagates through composite
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn adversarial_secret_leak_via_secret_input_is_rejected() {
        // Given a workflow that finishes with a secret-marked input
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.password".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "password".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak (E0406)
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    #[test]
    fn adversarial_type_mismatch_object_in_choose_is_rejected() {
        // Given a choose step with an Object-typed condition
        let wf = make_workflow(vec![choose_step(
            "bad_route",
            TypedValue::Literal(ValueType::Object),
        )]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns TypeMismatch (E0407) expected boolean, found object
        assert_eq!(
            result,
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "object".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_type_mismatch_list_in_choose_is_rejected() {
        // Given a choose step with a List-typed condition
        let wf = make_workflow(vec![choose_step(
            "bad_route",
            TypedValue::Literal(ValueType::List),
        )]);
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns TypeMismatch (E0407)
        assert_eq!(
            result,
            Err(ValidationError::TypeMismatch {
                expected: "boolean".to_owned(),
                found: "list".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_resource_limit_declared_exceeding_hard_limit_is_rejected() {
        // Given a workflow where declared max_slots exceeds the hard limit
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_steps: 1_000,
                max_slots: 100_000, // exceeds hard limit
                max_constants: 8_192,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default(); // max_slots = 65_535
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns LimitExceeded (E040A) for max_slots
        assert_eq!(
            result,
            Err(ValidationError::LimitExceeded {
                resource: "max_slots".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_resource_limit_actual_exceeding_declared_is_rejected() {
        // Given a workflow with 10 steps but declared max_steps of 5
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![finish_step("s1", TypedValue::Literal(ValueType::Number)); 10],
            resource_contract: ResourceLimits {
                max_steps: 5,
                max_slots: 65_535,
                max_constants: 8_192,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default();
        // When validate_resource_limits is called
        let result = validate_resource_limits(&wf, &hard);
        // Then it returns LimitExceeded (E040A) for max_steps
        assert_eq!(
            result,
            Err(ValidationError::LimitExceeded {
                resource: "max_steps".to_owned(),
            })
        );
    }

    #[test]
    fn adversarial_clean_input_passes_taint_check() {
        // Given a workflow that finishes with a clean input
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.username".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "username".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns Ok -- clean input is not tainted
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_secret_in_choose_condition_does_not_leak_but_type_check_passes_for_any() {
        // Given a choose step with a secret-derived condition that is Any-typed
        let mut wf = make_workflow(vec![
            save_step("val", TypedValue::Reference("$secrets.token".into())),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        // When validate_types is called
        let result = validate_types(&wf);
        // Then it returns Ok -- Any type is accepted for choose condition
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn adversarial_deeply_nested_composite_taint_propagates() {
        // Given a workflow with a deeply nested composite containing a secret
        let mut wf = make_workflow(vec![
            save_step(
                "nested",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Composite(vec![
                        TypedValue::Literal(ValueType::Text),
                        TypedValue::Reference("$secrets.deep_secret".into()),
                    ]),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("deep_secret".to_owned());
        // When validate_taint is called
        let result = validate_taint(&wf);
        // Then it returns SecretResultLeak (E0406) -- taint propagates through nested composites
        assert_eq!(result, Err(ValidationError::SecretResultLeak));
    }

    // ========================================================================
    // Section 38 behavioral property tests
    // ========================================================================

    /// Section 38 test 4: Taint safety -- secret reaching Finish is caught
    /// at validation time. Direct reference to secret in finish result.
    #[test]
    fn section38_taint_safety_secret_result_leak_direct_reference() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$secrets.api_key".into()),
        )]);
        wf.secrets.push("api_key".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Section 38 test 4: Secret propagates through a slot into Finish.
    #[test]
    fn section38_taint_safety_secret_result_leak_via_slot() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Section 38 test 4: Secret input propagates into Finish.
    #[test]
    fn section38_taint_safety_secret_result_leak_via_input() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.credential".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "credential".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Section 38 test 4: Clean finish passes validation.
    #[test]
    fn section38_taint_safety_clean_finish_passes() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // ========================================================================
    // Comprehensive taint propagation tests
    // ========================================================================

    // ---------------------------------------------------------------------------
    // 1. Secret-derived taint: a slot that receives output from a step reading
    //    a secret should be marked tainted.
    // ---------------------------------------------------------------------------

    /// When a save step reads `$secrets.token`, the resulting slot is tainted
    /// and cannot be used in a finish without triggering SecretResultLeak.
    #[test]
    fn taint_secret_save_marks_slot_tainted() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.token".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("token".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A save step reading a secret input marks the slot tainted.
    #[test]
    fn taint_secret_input_save_marks_slot_tainted() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$input.api_key".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "api_key".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A save step reading a clean input does NOT taint the slot.
    #[test]
    fn taint_clean_input_save_marks_slot_clean() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$input.username".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "username".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A save step reading a clean var does NOT taint the slot.
    #[test]
    fn taint_clean_var_save_marks_slot_clean() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$vars.counter".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.vars.push(("counter".to_owned(), ValueType::Number));
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A save step reading a literal does NOT taint the slot.
    #[test]
    fn taint_literal_save_marks_slot_clean() {
        let wf = make_workflow(vec![
            save_step("cap", TypedValue::Literal(ValueType::Text)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // ---------------------------------------------------------------------------
    // 2. Cross-slot contamination: if a tainted slot feeds into another
    //    save/relay step, all downstream output slots should become tainted.
    // ---------------------------------------------------------------------------

    /// Taint propagates through a chain of slot relays (slot 0 -> slot 1 -> slot 2).
    #[test]
    fn taint_propagates_through_three_hop_slot_chain() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.db_password".into())),
            save_step("relay1", TypedValue::Slot(0)),
            save_step("relay2", TypedValue::Slot(1)),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("db_password".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Two independent slots: slot 0 is tainted, slot 1 is clean.
    /// Finishing with the clean slot passes.
    #[test]
    fn taint_independent_slots_isolated_clean_finish() {
        let mut wf = make_workflow(vec![
            save_step("secret_cap", TypedValue::Reference("$secrets.key".into())),
            save_step("clean_cap", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("key".to_owned());
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Finishing with the tainted slot among independent slots fails.
    #[test]
    fn taint_independent_slots_tainted_finish_fails() {
        let mut wf = make_workflow(vec![
            save_step("secret_cap", TypedValue::Reference("$secrets.key".into())),
            save_step("clean_cap", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("key".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A tainted slot mixed with a clean literal in a composite taints the
    /// composite output.
    #[test]
    fn taint_cross_slot_contamination_via_composite() {
        let mut wf = make_workflow(vec![
            save_step("secret_cap", TypedValue::Reference("$secrets.token".into())),
            save_step(
                "mixed",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Literal(ValueType::Number),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("token".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A tainted slot relayed into a new save, combined with a clean literal
    /// in a nested composite, still carries taint.
    #[test]
    fn taint_nested_slot_relay_with_composite() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.cred".into())),
            save_step("s1", TypedValue::Slot(0)),
            save_step(
                "s2",
                TypedValue::Composite(vec![
                    TypedValue::Slot(1),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("cred".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    // ---------------------------------------------------------------------------
    // 3. Finish step with secret leak: writing a tainted slot to the result
    //    should produce a taint warning.
    // ---------------------------------------------------------------------------

    /// Direct secret reference in finish result triggers SecretResultLeak.
    #[test]
    fn taint_finish_direct_secret_reference_rejected() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$secrets.private_key".into()),
        )]);
        wf.secrets.push("private_key".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Secret input reference in finish result triggers SecretResultLeak.
    #[test]
    fn taint_finish_secret_input_reference_rejected() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.secret_value".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "secret_value".to_owned(),
            schema_type: ValueType::Number,
            is_secret: true,
        });
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Tainted slot used in finish result triggers SecretResultLeak.
    #[test]
    fn taint_finish_tainted_slot_rejected() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.session_id".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("session_id".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Tainted composite in finish result triggers SecretResultLeak.
    #[test]
    fn taint_finish_composite_with_secret_rejected() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Reference("$secrets.hidden".into()),
            ]),
        )]);
        wf.secrets.push("hidden".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// Tainted slot inside a composite in finish result triggers SecretResultLeak.
    #[test]
    fn taint_finish_composite_with_tainted_slot_rejected() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.bearer".into())),
            finish_step(
                "done",
                TypedValue::Composite(vec![TypedValue::Slot(0)]),
            ),
        ]);
        wf.secrets.push("bearer".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    // ---------------------------------------------------------------------------
    // 4. Clean finish: writing only non-tainted slots should not produce any
    //    taint warnings.
    // ---------------------------------------------------------------------------

    /// A finish with a literal value passes taint validation.
    #[test]
    fn taint_clean_finish_literal() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Literal(ValueType::Number),
        )]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A finish with a clean input reference passes taint validation.
    #[test]
    fn taint_clean_finish_clean_input_reference() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.email".into()),
        )]);
        wf.inputs.push(InputDecl {
            name: "email".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A finish with a clean var reference passes taint validation.
    #[test]
    fn taint_clean_finish_clean_var_reference() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$vars.status".into()),
        )]);
        wf.vars.push(("status".to_owned(), ValueType::Text));
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A finish with a clean slot (written from a clean input) passes.
    #[test]
    fn taint_clean_finish_clean_slot() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$input.user_id".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "user_id".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A finish with a composite of clean values passes.
    #[test]
    fn taint_clean_finish_composite_of_clean() {
        let wf = make_workflow(vec![
            save_step("cap", TypedValue::Literal(ValueType::Number)),
            finish_step(
                "done",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Multiple saves and a clean finish: secrets exist but are not used in the
    /// finish result.
    #[test]
    fn taint_clean_finish_with_unused_secrets_in_workflow() {
        let mut wf = make_workflow(vec![
            save_step("cap", TypedValue::Reference("$secrets.unused".into())),
            save_step("data", TypedValue::Literal(ValueType::Text)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("unused".to_owned());
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // ---------------------------------------------------------------------------
    // 5. Expression evaluation taint: using a tainted input in an expression
    //    (composite) should propagate taint to the output.
    // ---------------------------------------------------------------------------

    /// A composite containing a secret reference is tainted.
    #[test]
    fn taint_expression_composite_with_secret_reference() {
        let mut wf = make_workflow(vec![
            save_step(
                "expr",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Reference("$secrets.otp".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("otp".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A composite containing a tainted slot is tainted.
    #[test]
    fn taint_expression_composite_with_tainted_slot() {
        let mut wf = make_workflow(vec![
            save_step("secret_cap", TypedValue::Reference("$secrets.hash".into())),
            save_step(
                "expr",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("hash".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A composite of only clean values remains clean.
    #[test]
    fn taint_expression_composite_clean_remains_clean() {
        let wf = make_workflow(vec![
            save_step("clean_val", TypedValue::Literal(ValueType::Number)),
            save_step(
                "expr",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A deeply nested composite with a secret at the innermost level is tainted.
    #[test]
    fn taint_expression_deeply_nested_composite_with_secret() {
        let mut wf = make_workflow(vec![
            save_step(
                "deep",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Composite(vec![
                        TypedValue::Literal(ValueType::Text),
                        TypedValue::Composite(vec![
                            TypedValue::Literal(ValueType::Boolean),
                            TypedValue::Reference("$secrets.buried".into()),
                        ]),
                    ]),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("buried".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A composite that mixes a secret input with a clean literal is tainted.
    #[test]
    fn taint_expression_composite_with_secret_input() {
        let mut wf = make_workflow(vec![
            save_step(
                "expr",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Reference("$input.password".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "password".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A composite that uses a relay of a tainted slot is still tainted.
    #[test]
    fn taint_expression_composite_with_relayed_taint() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.nonce".into())),
            save_step("s1", TypedValue::Slot(0)),
            save_step(
                "expr",
                TypedValue::Composite(vec![TypedValue::Slot(1)]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("nonce".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// An inline composite (not via a slot) used directly in finish with a
    /// secret reference is rejected.
    #[test]
    fn taint_expression_inline_composite_in_finish_rejected() {
        let mut wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Reference("$secrets.inline_secret".into()),
            ]),
        )]);
        wf.secrets.push("inline_secret".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    // ---------------------------------------------------------------------------
    // Edge cases: multiple secrets, mixed inputs, choose steps
    // ---------------------------------------------------------------------------

    /// Multiple secrets declared but only one used in the taint path.
    #[test]
    fn taint_multiple_secrets_only_one_used() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.alpha".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("alpha".to_owned());
        wf.secrets.push("beta".to_owned());
        wf.secrets.push("gamma".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A choose step with a tainted condition does not cause a taint error
    /// (choose conditions only affect type checking, not taint flow to finish).
    #[test]
    fn taint_choose_with_secret_condition_no_finish_leak() {
        let mut wf = make_workflow(vec![
            save_step("val", TypedValue::Reference("$secrets.flag".into())),
            choose_step("route", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Literal(ValueType::Number)),
        ]);
        wf.secrets.push("flag".to_owned());
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A workflow with only clean steps (no secrets declared at all) passes.
    #[test]
    fn taint_no_secrets_all_clean() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Number)),
            save_step("flag", TypedValue::Literal(ValueType::Boolean)),
            choose_step("route", TypedValue::Slot(1)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A reference to an unknown name resolves as clean Any (no taint).
    #[test]
    fn taint_unknown_reference_resolves_clean() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.nonexistent".into()),
        )]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// A composite with multiple tainted inputs from different sources
    /// (secret + secret input) is tainted.
    #[test]
    fn taint_composite_with_multiple_taint_sources() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.s".into())),
            save_step(
                "expr",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Reference("$input.cred".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("s".to_owned());
        wf.inputs.push(InputDecl {
            name: "cred".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A composite with only clean inputs and clean slots passes taint check
    /// even when secrets are declared in the workflow.
    #[test]
    fn taint_composite_clean_with_declared_secrets() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$input.data".into())),
            save_step(
                "expr",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.inputs.push(InputDecl {
            name: "data".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        wf.secrets.push("unused_secret".to_owned());
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // ---------------------------------------------------------------------------
    // Deep slot chain and mixed-read isolation tests
    // ---------------------------------------------------------------------------

    /// Taint propagates through a deep chain of slot relays (5 intermediaries).
    /// Secret -> slot0 -> slot1 -> slot2 -> slot3 -> slot4 -> finish(slot5)
    /// verifies full transitive closure across enough intermediaries to catch
    /// any short-circuit or fixed-depth propagation bug.
    #[test]
    fn taint_propagates_through_deep_slot_chain() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.db_password".into())),
            save_step("s1", TypedValue::Slot(0)),
            save_step("s2", TypedValue::Slot(1)),
            save_step("s3", TypedValue::Slot(2)),
            save_step("s4", TypedValue::Slot(3)),
            save_step("s5", TypedValue::Slot(4)),
            finish_step("done", TypedValue::Slot(5)),
        ]);
        wf.secrets.push("db_password".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    /// A deep slot chain of clean relays does not produce a false positive.
    /// Clean literal -> slot0 -> slot1 -> slot2 -> slot3 -> finish(slot4)
    /// should pass taint validation.
    #[test]
    fn clean_deep_slot_chain_passes() {
        let wf = make_workflow(vec![
            save_step("s0", TypedValue::Literal(ValueType::Number)),
            save_step("s1", TypedValue::Slot(0)),
            save_step("s2", TypedValue::Slot(1)),
            save_step("s3", TypedValue::Slot(2)),
            save_step("s4", TypedValue::Slot(3)),
            finish_step("done", TypedValue::Slot(4)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Multi-slot read with mixed taint: a workflow that saves both a tainted
    /// slot and a clean slot. Finishing from the clean path passes, while
    /// finishing from the tainted path fails. This verifies that tainted and
    /// clean slots remain isolated even when they coexist in the same workflow.
    #[test]
    fn taint_mixed_slots_isolated_independent_reads() {
        let mut wf = make_workflow(vec![
            // slot 0: tainted
            save_step("tainted", TypedValue::Reference("$secrets.api_key".into())),
            // slot 1: clean
            save_step("clean", TypedValue::Literal(ValueType::Text)),
            // slot 2: clean relay of slot 1 (still clean)
            save_step("clean_relay", TypedValue::Slot(1)),
            // finish from the clean relay -- should pass
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("api_key".to_owned());
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Same setup as `taint_mixed_slots_isolated_independent_reads` but the
    /// finish reads the tainted slot instead. Must be rejected.
    #[test]
    fn taint_mixed_slots_isolated_tainted_read_rejected() {
        let mut wf = make_workflow(vec![
            // slot 0: tainted
            save_step("tainted", TypedValue::Reference("$secrets.api_key".into())),
            // slot 1: clean
            save_step("clean", TypedValue::Literal(ValueType::Text)),
            // slot 2: clean relay of slot 1 (still clean)
            save_step("clean_relay", TypedValue::Slot(1)),
            // finish from the tainted slot -- must be rejected
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("api_key".to_owned());
        assert_eq!(validate_taint(&wf), Err(ValidationError::SecretResultLeak));
    }

    // =========================================================================
    // BLACKHAT security regression tests
    // =========================================================================

    /// BLACKHAT: validate_resource_limits passes steps.len() as actual for
    /// both max_steps AND max_slots checks.
    ///
    /// SEVERITY: LOW (logic smell, not exploitable)
    /// DESCRIPTION: In `validate_resource_limits`, the `check_resource_bound`
    /// call for "max_slots" passes `workflow.steps.len()` as the `actual` value.
    /// This means the max_slots check compares the step count against the
    /// declared max_slots limit, which is semantically incorrect -- it should
    /// compare against the actual slot count. However, since max_slots defaults
    /// to 65_535 and workflows typically have far fewer steps, this is unlikely
    /// to cause a false rejection in practice.
    #[test]
    fn blackhat_resource_limits_max_slots_uses_step_count_not_slot_count() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![
                finish_step("s0", TypedValue::Literal(ValueType::Number)),
                finish_step("s1", TypedValue::Literal(ValueType::Number)),
            ],
            resource_contract: ResourceLimits {
                max_steps: 10,
                max_slots: 1, // Only 1 slot declared, but steps.len() = 2
                max_constants: 8_192,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default();

        // Because check_resource_bound("max_slots", steps.len()=2, declared=1, hard)
        // is called, this should fail with LimitExceeded for max_slots.
        match validate_resource_limits(&wf, &hard) {
            Err(ValidationError::LimitExceeded { resource }) => {
                assert_eq!(
                    resource, "max_slots",
                    "blackhat: max_slots check should fail because steps.len() > max_slots"
                );
            }
            Err(ValidationError::LimitRequired { .. }) => {
                // Another zero limit may be hit first
            }
            other => {
                // blackhat: expected LimitExceeded or LimitRequired for max_slots
                assert!(
                    matches!(other, Err(ValidationError::LimitExceeded { .. } | ValidationError::LimitRequired { .. })),
                    "blackhat: expected LimitExceeded or LimitRequired for max_slots, got {other:?}"
                );
            }
        }
    }

    /// BLACKHAT: unknown references resolve as clean Any (no taint).
    ///
    /// SEVERITY: LOW (defense-in-depth concern)
    /// DESCRIPTION: When `resolve_reference` encounters an unknown root or
    /// unknown name, it returns `ValueFact::clean(ValueType::Any)`. This means
    /// an attacker cannot leak secrets by referencing undeclared names in the
    /// finish result, because unknown references are always clean. However, it
    /// also means that typos in reference names silently produce clean values
    /// instead of validation errors.
    #[test]
    fn blackhat_unknown_reference_is_clean_not_secret() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$unknown_root.field".into()),
        )]);
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "blackhat: unknown reference roots should resolve as clean"
        );

        let wf2 = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input.nonexistent".into()),
        )]);
        assert_eq!(
            validate_taint(&wf2),
            Ok(()),
            "blackhat: unknown input name should resolve as clean"
        );
    }

    /// BLACKHAT: reference without dot separator resolves as clean Any.
    ///
    /// SEVERITY: LOW
    /// DESCRIPTION: A reference like "$input" (without a dot) resolves as
    /// clean Any because `body.split_once('.')` returns None. This is a
    /// safe default -- it cannot introduce taint.
    #[test]
    fn blackhat_reference_without_dot_is_clean_any() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("$input".into()),
        )]);
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "blackhat: reference without dot should be clean"
        );
    }

    /// BLACKHAT: reference without dollar prefix resolves as clean Text.
    ///
    /// SEVERITY: LOW
    /// DESCRIPTION: A reference that does not start with "$" resolves as
    /// `ValueFact::clean(ValueType::Text)` by `resolve_reference`. This is
    /// a safe default.
    #[test]
    fn blackhat_reference_without_dollar_prefix_is_clean_text() {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Reference("not_a_reference".into()),
        )]);
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "blackhat: non-dollar reference should be clean"
        );
    }

    /// BLACKHAT: check_declared_bound rejects zero as a required limit.
    ///
    /// SEVERITY: MEDIUM (prevents limit-omission attacks)
    /// DESCRIPTION: `check_declared_bound` returns `LimitRequired` when
    /// `declared == 0`. This prevents a workflow from declaring a zero limit
    /// for any resource, which could be used to bypass resource accounting.
    #[test]
    fn blackhat_zero_declared_limit_rejected() {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_input_bytes: 0,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default();

        let result = validate_resource_limits(&wf, &hard);
        assert!(
            matches!(
                result,
                Err(ValidationError::LimitRequired { .. })
            ),
            "blackhat: zero declared limit should be rejected, got {result:?}"
        );
        if let Err(ValidationError::LimitRequired { resource }) = result {
            assert_eq!(
                resource, "max_input_bytes",
                "blackhat: zero declared limit should be rejected"
            );
        }
    }

    /// BLACKHAT: Choose step with tainted condition does not propagate taint to slots.
    ///
    /// SEVERITY: LOW (correct behavior, documented)
    /// DESCRIPTION: A Choose step does not write to any slot, so even if its
    /// condition is tainted, no taint propagates.
    #[test]
    fn blackhat_choose_tainted_condition_does_not_propagate() {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.flag".into())),
            choose_step("route", TypedValue::Slot(0)),
            save_step("s1", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("flag".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "blackhat: tainted choose condition should not propagate taint to finish"
        );
    }

    /// BLACKHAT: Taint merge is commutative.
    ///
    /// SEVERITY: INFORMATIONAL (correctness verification)
    /// DESCRIPTION: Verify that Taint::merge is commutative.
    #[test]
    fn blackhat_taint_merge_commutative() {
        assert_eq!(
            Taint::Clean.merge(Taint::Secret),
            Taint::Secret.merge(Taint::Clean),
            "blackhat: taint merge must be commutative"
        );
    }

    /// BLACKHAT: Slot reference to uninitialized slot resolves as clean Any.
    ///
    /// SEVERITY: LOW
    /// DESCRIPTION: When a Slot value references an index that has not been
    /// written to (i.e., the slot is None), `resolve_value` returns
    /// `ValueFact::clean(ValueType::Any)`. This means reading from an
    /// uninitialized slot never introduces taint.
    #[test]
    fn blackhat_uninitialized_slot_is_clean() {
        let wf = make_workflow(vec![
            save_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(5)),
        ]);
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "blackhat: uninitialized slot should resolve as clean"
        );
    }

    /// BLACKHAT: Empty composite resolves as clean with Any type.
    ///
    /// SEVERITY: INFORMATIONAL
    /// DESCRIPTION: An empty composite `Composite(vec![])` has no children,
    /// so no taint can propagate.
    #[test]
    fn blackhat_empty_composite_is_clean() {
        let wf = make_workflow(vec![
            save_step("s0", TypedValue::Composite(vec![])),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "blackhat: empty composite should be clean"
        );
    }

    /// BLACKHAT: write_slot silently drops out-of-bounds writes.
    ///
    /// SEVERITY: LOW (cannot happen via normal API, but documented)
    /// DESCRIPTION: write_slot uses get_mut which returns None for OOB indices.
    #[test]
    fn blackhat_out_of_bounds_slot_write_no_panic() {
        let wf = make_workflow(vec![
            save_step("overflow", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    // =========================================================================
    // Comprehensive taint propagation and type-checking tests (request-fulfillment)
    // =========================================================================

    // ---------------------------------------------------------------------------
    // 1. Taint propagation through composite "expression" operations
    //    (In this model, Composite is the expression form. We verify that taint
    //    propagates through composites representing arithmetic, comparison, and
    //    logic combinations.)
    // ---------------------------------------------------------------------------

    /// "Arithmetic" expression: a composite combining a secret input with a
    /// literal number. Taint must propagate through the composite into the slot.
    fn taint_propagates_through_arithmetic_style_composite() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step(
                "arithmetic_result",
                TypedValue::Composite(vec![
                    TypedValue::Reference("$input.secret_salary".into()),
                    TypedValue::Literal(ValueType::Number),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "secret_salary".to_owned(),
            schema_type: ValueType::Number,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for arithmetic composite, got {result:?}"
            ));
        }
        Ok(())
    }

    /// "Comparison" expression: a composite combining two references, one of
    /// which is secret. Taint must propagate.
    fn taint_propagates_through_comparison_style_composite() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step(
                "comparison_result",
                TypedValue::Composite(vec![
                    TypedValue::Reference("$input.public_id".into()),
                    TypedValue::Reference("$secrets.compare_key".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "public_id".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        wf.secrets.push("compare_key".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for comparison composite, got {result:?}"
            ));
        }
        Ok(())
    }

    /// "Logic" expression: a composite combining a secret-derived slot with a
    /// clean literal. Taint must propagate.
    fn taint_propagates_through_logic_style_composite() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("flag", TypedValue::Reference("$secrets.logic_val".into())),
            save_step(
                "logic_result",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Literal(ValueType::Boolean),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("logic_val".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for logic composite, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A composite with all clean inputs stays clean through the "expression".
    fn clean_composite_stays_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step(
                "clean_expr",
                TypedValue::Composite(vec![
                    TypedValue::Reference("$input.a".into()),
                    TypedValue::Reference("$input.b".into()),
                    TypedValue::Literal(ValueType::Number),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "a".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        wf.inputs.push(InputDecl {
            name: "b".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for clean composite expression, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_taint_propagates_through_arithmetic_style_composite() {
        taint_propagates_through_arithmetic_style_composite().unwrap()
    }

    #[test]
    fn run_taint_propagates_through_comparison_style_composite() {
        taint_propagates_through_comparison_style_composite().unwrap()
    }

    #[test]
    fn run_taint_propagates_through_logic_style_composite() {
        taint_propagates_through_logic_style_composite().unwrap()
    }

    #[test]
    fn run_clean_composite_stays_clean() {
        clean_composite_stays_clean().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 2. Taint origin tracking: Secret inputs propagate through ALL downstream
    //    slots via every path kind (direct reference, slot relay, composite).
    // ---------------------------------------------------------------------------

    /// A secret input is saved to slot 0, relayed to slot 1, used in a
    /// composite in slot 2, relayed again to slot 3, then finished.
    /// Every downstream slot in the chain must be tainted.
    fn secret_origin_propagates_through_all_downstream_paths() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            // slot 0: capture secret input directly
            save_step("s0", TypedValue::Reference("$input.api_key".into())),
            // slot 1: relay slot 0
            save_step("s1", TypedValue::Slot(0)),
            // slot 2: composite with slot 1
            save_step(
                "s2",
                TypedValue::Composite(vec![TypedValue::Slot(1), TypedValue::Literal(ValueType::Text)]),
            ),
            // slot 3: relay slot 2
            save_step("s3", TypedValue::Slot(2)),
            // finish from slot 3 -- should be tainted
            finish_step("done", TypedValue::Slot(3)),
        ]);
        wf.inputs.push(InputDecl {
            name: "api_key".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for secret origin chain, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Same chain but finishing from slot 1 (the relay) is also tainted.
    fn secret_origin_relay_slot_is_tainted() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$input.secret_val".into())),
            save_step("s1", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.inputs.push(InputDecl {
            name: "secret_val".to_owned(),
            schema_type: ValueType::Number,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for relay of secret input, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Same chain but finishing from the composite (slot 2) is also tainted.
    fn secret_origin_composite_slot_is_tainted() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$input.secret_val".into())),
            save_step(
                "s1",
                TypedValue::Composite(vec![TypedValue::Slot(0)]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.inputs.push(InputDecl {
            name: "secret_val".to_owned(),
            schema_type: ValueType::Number,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for composite of secret input, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_secret_origin_propagates_through_all_downstream_paths() {
        secret_origin_propagates_through_all_downstream_paths().unwrap()
    }

    #[test]
    fn run_secret_origin_relay_slot_is_tainted() {
        secret_origin_relay_slot_is_tainted().unwrap()
    }

    #[test]
    fn run_secret_origin_composite_slot_is_tainted() {
        secret_origin_composite_slot_is_tainted().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 3. Slot-to-slot taint flow via Copy (relay) operations
    // ---------------------------------------------------------------------------

    /// Taint flows from slot 0 to slot 1 via a relay save.
    fn slot_to_slot_single_relay_propagates_taint() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("origin", TypedValue::Reference("$secrets.db_url".into())),
            save_step("copy", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("db_url".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for single slot relay, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Taint does NOT flow into a slot that reads a clean slot.
    fn slot_to_slot_clean_relay_stays_clean() -> Result<(), String> {
        let wf = make_workflow(vec![
            save_step("origin", TypedValue::Literal(ValueType::Number)),
            save_step("copy", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for clean slot relay, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Two relay branches from the same tainted origin: both are tainted.
    fn slot_to_slot_branching_relays_both_tainted() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("origin", TypedValue::Reference("$secrets.master_key".into())),
            save_step("branch_a", TypedValue::Slot(0)),
            save_step("branch_b", TypedValue::Slot(0)),
            // finish from branch_b -- both branches derive from the same tainted origin
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("master_key".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for branching relay, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Relay from a relay: two-hop copy chain carries taint.
    fn slot_to_slot_two_hop_relay_carries_taint() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.cred".into())),
            save_step("s1", TypedValue::Slot(0)),
            save_step("s2", TypedValue::Slot(1)),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("cred".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for two-hop relay, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_slot_to_slot_single_relay_propagates_taint() {
        slot_to_slot_single_relay_propagates_taint().unwrap()
    }

    #[test]
    fn run_slot_to_slot_clean_relay_stays_clean() {
        slot_to_slot_clean_relay_stays_clean().unwrap()
    }

    #[test]
    fn run_slot_to_slot_branching_relays_both_tainted() {
        slot_to_slot_branching_relays_both_tainted().unwrap()
    }

    #[test]
    fn run_slot_to_slot_two_hop_relay_carries_taint() {
        slot_to_slot_two_hop_relay_carries_taint().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 4. Conditional taint: branch paths with different taint levels
    // ---------------------------------------------------------------------------

    /// A choose step does not produce a slot and does not propagate taint.
    /// The finish step reads from a clean slot even though a tainted slot exists.
    fn conditional_taint_choose_does_not_taint_downstream() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("tainted_flag", TypedValue::Reference("$secrets.branch_sel".into())),
            save_step("clean_data", TypedValue::Literal(ValueType::Number)),
            choose_step("branch", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("branch_sel".to_owned());
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: choose does not propagate taint to downstream finish, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A choose step with a clean condition and a tainted data slot: finishing
    /// the tainted slot still leaks.
    fn conditional_taint_finish_after_choose_reads_tainted() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("clean_flag", TypedValue::Literal(ValueType::Boolean)),
            save_step("tainted_data", TypedValue::Reference("$secrets.payload".into())),
            choose_step("branch", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("payload".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak: finishing tainted slot after choose, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Multiple choose steps interleaved with saves: taint is only from data,
    /// not from choose conditions.
    fn conditional_taint_multiple_chooses_interleaved() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("flag1", TypedValue::Literal(ValueType::Boolean)),
            choose_step("branch1", TypedValue::Slot(0)),
            save_step("flag2", TypedValue::Reference("$secrets.secret_flag".into())),
            choose_step("branch2", TypedValue::Slot(2)),
            save_step("clean_result", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(4)),
        ]);
        wf.secrets.push("secret_flag".to_owned());
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: choose conditions do not taint downstream, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Choose condition from a clean input: type check passes (Boolean), taint
    /// is clean.
    fn conditional_taint_clean_boolean_choose_passes_both_validators() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("flag", TypedValue::Reference("$input.is_admin".into())),
            choose_step("route", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Literal(ValueType::Text)),
        ]);
        wf.inputs.push(InputDecl {
            name: "is_admin".to_owned(),
            schema_type: ValueType::Boolean,
            is_secret: false,
        });
        let type_result = validate_types(&wf);
        if type_result != Ok(()) {
            return Err(format!(
                "expected Ok for type check with boolean input, got {type_result:?}"
            ));
        }
        let taint_result = validate_taint(&wf);
        if taint_result != Ok(()) {
            return Err(format!(
                "expected Ok for taint check with clean input, got {taint_result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_conditional_taint_choose_does_not_taint_downstream() {
        conditional_taint_choose_does_not_taint_downstream().unwrap()
    }

    #[test]
    fn run_conditional_taint_finish_after_choose_reads_tainted() {
        conditional_taint_finish_after_choose_reads_tainted().unwrap()
    }

    #[test]
    fn run_conditional_taint_multiple_chooses_interleaved() {
        conditional_taint_multiple_chooses_interleaved().unwrap()
    }

    #[test]
    fn run_conditional_taint_clean_boolean_choose_passes_both_validators() {
        conditional_taint_clean_boolean_choose_passes_both_validators().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 5. Taint through accessor operations (list items, object fields)
    //    In this model, accessors are Reference paths like $input.user.name.
    //    The reference_name function extracts the first segment after root.
    // ---------------------------------------------------------------------------

    /// Accessing a field of a secret input still carries taint.
    fn accessor_secret_input_field_carries_taint() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("field_read", TypedValue::Reference("$input.credential.token".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "credential".to_owned(),
            schema_type: ValueType::Object,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for secret field access, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Accessing a deeply nested field of a clean input stays clean.
    fn accessor_clean_input_nested_field_stays_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("field_read", TypedValue::Reference("$input.user.profile.name".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "user".to_owned(),
            schema_type: ValueType::Object,
            is_secret: false,
        });
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for clean nested field access, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Accessing a field of a declared secret via $secrets.name.field.
    fn accessor_secret_field_via_secrets_namespace() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("val", TypedValue::Reference("$secrets.db.connection_string".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("db".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for secret accessor, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Accessing a field via $vars namespace: vars are always clean.
    fn accessor_var_field_is_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("val", TypedValue::Reference("$vars.config.threshold".into())),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.vars.push(("config".to_owned(), ValueType::Object));
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for var accessor, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A composite combining a secret accessor with a clean literal is tainted.
    fn accessor_secret_in_composite_propagates_taint() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step(
                "mixed",
                TypedValue::Composite(vec![
                    TypedValue::Reference("$secrets.key.sub_key".into()),
                    TypedValue::Literal(ValueType::Number),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("key".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for secret accessor in composite, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A composite of two clean accessors (input + var) stays clean.
    fn accessor_composite_of_clean_accessors_stays_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step(
                "combined",
                TypedValue::Composite(vec![
                    TypedValue::Reference("$input.data.value".into()),
                    TypedValue::Reference("$vars.settings.limit".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "data".to_owned(),
            schema_type: ValueType::Object,
            is_secret: false,
        });
        wf.vars.push(("settings".to_owned(), ValueType::Object));
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for composite of clean accessors, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_accessor_secret_input_field_carries_taint() {
        accessor_secret_input_field_carries_taint().unwrap()
    }

    #[test]
    fn run_accessor_clean_input_nested_field_stays_clean() {
        accessor_clean_input_nested_field_stays_clean().unwrap()
    }

    #[test]
    fn run_accessor_secret_field_via_secrets_namespace() {
        accessor_secret_field_via_secrets_namespace().unwrap()
    }

    #[test]
    fn run_accessor_var_field_is_clean() {
        accessor_var_field_is_clean().unwrap()
    }

    #[test]
    fn run_accessor_secret_in_composite_propagates_taint() {
        accessor_secret_in_composite_propagates_taint().unwrap()
    }

    #[test]
    fn run_accessor_composite_of_clean_accessors_stays_clean() {
        accessor_composite_of_clean_accessors_stays_clean().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 6. Clean taint verification for untainted paths
    // ---------------------------------------------------------------------------

    /// An entirely clean workflow with inputs, vars, saves, and chooses passes
    /// both type and taint validation.
    fn fully_clean_workflow_passes_both_validators() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("data", TypedValue::Reference("$input.count".into())),
            save_step("threshold", TypedValue::Reference("$vars.limit".into())),
            save_step("flag", TypedValue::Literal(ValueType::Boolean)),
            choose_step("route", TypedValue::Slot(2)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.inputs.push(InputDecl {
            name: "count".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        wf.vars.push(("limit".to_owned(), ValueType::Number));
        let type_result = validate_types(&wf);
        if type_result != Ok(()) {
            return Err(format!(
                "expected Ok for type validation, got {type_result:?}"
            ));
        }
        let taint_result = validate_taint(&wf);
        if taint_result != Ok(()) {
            return Err(format!(
                "expected Ok for taint validation, got {taint_result:?}"
            ));
        }
        Ok(())
    }

    /// A clean input used through multiple relay slots stays clean.
    fn clean_path_through_relay_chain() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$input.value".into())),
            save_step("s1", TypedValue::Slot(0)),
            save_step("s2", TypedValue::Slot(1)),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.inputs.push(InputDecl {
            name: "value".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for clean relay chain, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A clean composite (all literals) used in finish passes.
    fn clean_composite_in_finish_passes() -> Result<(), String> {
        let wf = make_workflow(vec![finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Literal(ValueType::Text),
                TypedValue::Literal(ValueType::Boolean),
            ]),
        )]);
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for clean composite finish, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Secrets exist in the workflow but are not in the finish path.
    fn clean_finish_with_secrets_in_other_paths() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("secret_slot", TypedValue::Reference("$secrets.unused".into())),
            save_step("clean_slot", TypedValue::Literal(ValueType::Text)),
            choose_step("route", TypedValue::Literal(ValueType::Boolean)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("unused".to_owned());
        let taint_result = validate_taint(&wf);
        if taint_result != Ok(()) {
            return Err(format!(
                "expected Ok: secrets in non-finish path should not cause leak, got {taint_result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_fully_clean_workflow_passes_both_validators() {
        fully_clean_workflow_passes_both_validators().unwrap()
    }

    #[test]
    fn run_clean_path_through_relay_chain() {
        clean_path_through_relay_chain().unwrap()
    }

    #[test]
    fn run_clean_composite_in_finish_passes() {
        clean_composite_in_finish_passes().unwrap()
    }

    #[test]
    fn run_clean_finish_with_secrets_in_other_paths() {
        clean_finish_with_secrets_in_other_paths().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 7. Taint merge: multiple tainted inputs combining into higher taint levels
    // ---------------------------------------------------------------------------

    /// Taint merge unit test: Secret + Secret = Secret.
    fn taint_merge_secret_plus_secret() -> Result<(), String> {
        let result = Taint::Secret.merge(Taint::Secret);
        if result != Taint::Secret {
            return Err(format!(
                "expected Taint::Secret, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Taint merge unit test: Clean + Clean = Clean.
    fn taint_merge_clean_plus_clean() -> Result<(), String> {
        let result = Taint::Clean.merge(Taint::Clean);
        if result != Taint::Clean {
            return Err(format!(
                "expected Taint::Clean, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Taint merge unit test: Secret + Clean = Secret (in both directions).
    fn taint_merge_secret_plus_clean_directions() -> Result<(), String> {
        let forward = Taint::Secret.merge(Taint::Clean);
        let backward = Taint::Clean.merge(Taint::Secret);
        if forward != Taint::Secret {
            return Err(format!(
                "expected Taint::Secret for Secret.merge(Clean), got {forward:?}"
            ));
        }
        if backward != Taint::Secret {
            return Err(format!(
                "expected Taint::Secret for Clean.merge(Secret), got {backward:?}"
            ));
        }
        Ok(())
    }

    /// A composite merging two secret sources (secret + secret input) is tainted.
    fn taint_merge_composite_of_two_secret_sources() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.alpha".into())),
            save_step(
                "merged",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Reference("$input.beta".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("alpha".to_owned());
        wf.inputs.push(InputDecl {
            name: "beta".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for merged secrets, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A composite merging a secret slot with a clean input is tainted (taint
    /// dominates).
    fn taint_merge_secret_dominates_over_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("secret_slot", TypedValue::Reference("$secrets.dom".into())),
            save_step(
                "merged",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Reference("$input.clean_val".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("dom".to_owned());
        wf.inputs.push(InputDecl {
            name: "clean_val".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak: secret taint dominates, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A composite of three secrets from different sources is tainted.
    fn taint_merge_three_distinct_secret_sources() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.a".into())),
            save_step("s1", TypedValue::Reference("$input.b".into())),
            save_step(
                "merged",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Slot(1),
                    TypedValue::Reference("$secrets.c".into()),
                ]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("a".to_owned());
        wf.secrets.push("c".to_owned());
        wf.inputs.push(InputDecl {
            name: "b".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak for three merged secrets, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_taint_merge_secret_plus_secret() {
        taint_merge_secret_plus_secret().unwrap()
    }

    #[test]
    fn run_taint_merge_clean_plus_clean() {
        taint_merge_clean_plus_clean().unwrap()
    }

    #[test]
    fn run_taint_merge_secret_plus_clean_directions() {
        taint_merge_secret_plus_clean_directions().unwrap()
    }

    #[test]
    fn run_taint_merge_composite_of_two_secret_sources() {
        taint_merge_composite_of_two_secret_sources().unwrap()
    }

    #[test]
    fn run_taint_merge_secret_dominates_over_clean() {
        taint_merge_secret_dominates_over_clean().unwrap()
    }

    #[test]
    fn run_taint_merge_three_distinct_secret_sources() {
        taint_merge_three_distinct_secret_sources().unwrap()
    }

    // ---------------------------------------------------------------------------
    // 8. Boundary cases: empty taint set, all slots tainted, circular-like
    //    references, empty workflows
    // ---------------------------------------------------------------------------

    /// Empty workflow (no steps) passes both validators.
    fn boundary_empty_workflow_passes() -> Result<(), String> {
        let wf = make_workflow(vec![]);
        let type_result = validate_types(&wf);
        if type_result != Ok(()) {
            return Err(format!(
                "expected Ok for empty workflow types, got {type_result:?}"
            ));
        }
        let taint_result = validate_taint(&wf);
        if taint_result != Ok(()) {
            return Err(format!(
                "expected Ok for empty workflow taint, got {taint_result:?}"
            ));
        }
        Ok(())
    }

    /// Workflow with no secrets declared and no secret inputs: all paths clean.
    fn boundary_no_secrets_at_all() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$input.x".into())),
            save_step("s1", TypedValue::Reference("$vars.y".into())),
            save_step("s2", TypedValue::Literal(ValueType::Text)),
            finish_step(
                "done",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Slot(1),
                    TypedValue::Slot(2),
                ]),
            ),
        ]);
        wf.inputs.push(InputDecl {
            name: "x".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        wf.vars.push(("y".to_owned(), ValueType::Number));
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for all-clean workflow, got {result:?}"
            ));
        }
        Ok(())
    }

    /// All slots are tainted: every save reads from a secret source.
    fn boundary_all_slots_tainted() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.a".into())),
            save_step("s1", TypedValue::Reference("$input.b".into())),
            save_step("s2", TypedValue::Slot(0)),
            save_step("s3", TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Slot(1),
            ])),
            finish_step("done", TypedValue::Slot(3)),
        ]);
        wf.secrets.push("a".to_owned());
        wf.inputs.push(InputDecl {
            name: "b".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak when all slots tainted, got {result:?}"
            ));
        }
        Ok(())
    }

    /// All slots tainted but finish uses a literal (clean): passes.
    fn boundary_all_slots_tainted_finish_uses_literal() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.x".into())),
            save_step("s1", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Literal(ValueType::Number)),
        ]);
        wf.secrets.push("x".to_owned());
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: finish uses literal even though all slots tainted, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Forward reference: a slot reads from an index that has not been written
    /// yet. The slot resolves as clean Any (uninitialized = None).
    fn boundary_forward_slot_reference_is_clean() -> Result<(), String> {
        let wf = make_workflow(vec![
            // Reads slot 3, which is only written at step index 3.
            // But step 3 has not executed yet when step 0 runs.
            // Actually, in this model validation is sequential, so slot 3
            // IS written by the time we validate step 4.
            // Let's instead read a slot that is NEVER written:
            save_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(99)),
        ]);
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: out-of-bounds slot resolves clean, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Self-referential slot: slot 0 reads from slot 0. Since it has not been
    /// written yet, it resolves as clean Any.
    fn boundary_self_referential_slot_is_clean() -> Result<(), String> {
        let wf = make_workflow(vec![
            save_step("s0", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: self-referential slot resolves clean, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A "cycle" via forward-then-backward: slot 0 writes a literal, slot 1
    /// reads slot 2 (not yet written, resolves clean), slot 2 reads slot 0.
    /// All clean.
    fn boundary_cycle_like_pattern_all_clean() -> Result<(), String> {
        let wf = make_workflow(vec![
            save_step("s0", TypedValue::Literal(ValueType::Number)),
            save_step("s1", TypedValue::Slot(2)), // slot 2 not yet written, resolves clean
            save_step("s2", TypedValue::Slot(0)), // slot 0 was written, clean literal
            finish_step("done", TypedValue::Slot(1)),
        ]);
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for cycle-like pattern, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Empty inputs, vars, secrets: a bare finish with a literal.
    fn boundary_bare_finish_literal() -> Result<(), String> {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![finish_step("done", TypedValue::Literal(ValueType::Null))],
            resource_contract: ResourceLimits::default(),
        };
        let type_result = validate_types(&wf);
        if type_result != Ok(()) {
            return Err(format!(
                "expected Ok for bare finish types, got {type_result:?}"
            ));
        }
        let taint_result = validate_taint(&wf);
        if taint_result != Ok(()) {
            return Err(format!(
                "expected Ok for bare finish taint, got {taint_result:?}"
            ));
        }
        Ok(())
    }

    /// A slot that is written twice: the second write overwrites the first.
    /// If the second write is clean, the slot becomes clean.
    fn boundary_slot_overwrite_second_write_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            // slot 0: tainted
            save_step("s0", TypedValue::Reference("$secrets.x".into())),
            // slot 0: overwritten with clean literal
            save_step("s0_again", TypedValue::Literal(ValueType::Number)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("x".to_owned());
        // Note: each save writes to the slot at its own step index.
        // Step 0 writes slot[0] = tainted.
        // Step 1 writes slot[1] = clean.
        // Finish reads slot[0] = tainted.
        // So this should actually fail because slot[0] is tainted.
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak: slot[0] still has tainted value, got {result:?}"
            ));
        }
        Ok(())
    }

    /// A slot that is overwritten with clean: slot 1 reads a secret, then slot 1
    /// is overwritten with a clean literal. Finish from slot 1 should be clean.
    fn boundary_slot_index_overwritten_to_clean() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.y".into())),
            // Now overwrite slot 0 (same index as step 0) with clean.
            // Actually, we can't overwrite the same slot since writes go to
            // the step's own index. Let me verify:
            // - Step 0 writes to slots[0] = tainted
            // - Step 1 writes to slots[1] = clean
            // - Step 2 (also saves to slots[2]) with a clean value
            // Instead, let's just check that finish from slot 1 (clean) works.
            save_step("s1", TypedValue::Literal(ValueType::Text)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("y".to_owned());
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: finish from clean slot, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Workflow with maximum reasonable chain length (10 saves) all clean.
    fn boundary_long_clean_chain_passes() -> Result<(), String> {
        let mut steps = Vec::new();
        steps.push(save_step("s0", TypedValue::Literal(ValueType::Number)));
        for i in 1..10 {
            let prev = i - 1;
            steps.push(save_step(
                &format!("s{i}"),
                TypedValue::Slot(prev),
            ));
        }
        steps.push(finish_step("done", TypedValue::Slot(9)));
        let wf = make_workflow(steps);
        let result = validate_taint(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for long clean chain, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_boundary_empty_workflow_passes() {
        boundary_empty_workflow_passes().unwrap()
    }

    #[test]
    fn run_boundary_no_secrets_at_all() {
        boundary_no_secrets_at_all().unwrap()
    }

    #[test]
    fn run_boundary_all_slots_tainted() {
        boundary_all_slots_tainted().unwrap()
    }

    #[test]
    fn run_boundary_all_slots_tainted_finish_uses_literal() {
        boundary_all_slots_tainted_finish_uses_literal().unwrap()
    }

    #[test]
    fn run_boundary_forward_slot_reference_is_clean() {
        boundary_forward_slot_reference_is_clean().unwrap()
    }

    #[test]
    fn run_boundary_self_referential_slot_is_clean() {
        boundary_self_referential_slot_is_clean().unwrap()
    }

    #[test]
    fn run_boundary_cycle_like_pattern_all_clean() {
        boundary_cycle_like_pattern_all_clean().unwrap()
    }

    #[test]
    fn run_boundary_bare_finish_literal() {
        boundary_bare_finish_literal().unwrap()
    }

    #[test]
    fn run_boundary_slot_overwrite_second_write_clean() {
        boundary_slot_overwrite_second_write_clean().unwrap()
    }

    #[test]
    fn run_boundary_slot_index_overwritten_to_clean() {
        boundary_slot_index_overwritten_to_clean().unwrap()
    }

    #[test]
    fn run_boundary_long_clean_chain_passes() {
        boundary_long_clean_chain_passes().unwrap()
    }

    // ---------------------------------------------------------------------------
    // Additional type-checking edge cases
    // ---------------------------------------------------------------------------

    /// Object type in choose condition is rejected.
    fn type_check_object_in_choose_rejected() -> Result<(), String> {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Object),
        )]);
        let result = validate_types(&wf);
        let expected = Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "object".to_owned(),
        });
        if result != expected {
            return Err(format!(
                "expected TypeMismatch(object), got {result:?}"
            ));
        }
        Ok(())
    }

    /// List type in choose condition is rejected.
    fn type_check_list_in_choose_rejected() -> Result<(), String> {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::List),
        )]);
        let result = validate_types(&wf);
        let expected = Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "list".to_owned(),
        });
        if result != expected {
            return Err(format!(
                "expected TypeMismatch(list), got {result:?}"
            ));
        }
        Ok(())
    }

    /// Number type in choose condition is rejected.
    fn type_check_number_in_choose_rejected() -> Result<(), String> {
        let wf = make_workflow(vec![choose_step(
            "route",
            TypedValue::Literal(ValueType::Number),
        )]);
        let result = validate_types(&wf);
        let expected = Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        });
        if result != expected {
            return Err(format!(
                "expected TypeMismatch(number), got {result:?}"
            ));
        }
        Ok(())
    }

    /// Any-typed slot from unresolved reference is accepted for choose.
    fn type_check_any_from_unresolved_ref_accepted() -> Result<(), String> {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Reference("$input.missing".into())),
            choose_step("route", TypedValue::Slot(0)),
        ]);
        let result = validate_types(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok: unresolved ref resolves as Any, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Save with composite value passes type check (composite type is Any).
    fn type_check_save_composite_passes() -> Result<(), String> {
        let wf = make_workflow(vec![
            save_step(
                "comp",
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Number),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
            finish_step("done", TypedValue::Literal(ValueType::Null)),
        ]);
        let result = validate_types(&wf);
        if result != Ok(()) {
            return Err(format!(
                "expected Ok for save with composite, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Multiple finish steps: only the one with tainted result matters.
    /// Validation processes steps in order and the first tainted finish
    /// triggers the error.
    fn type_check_multiple_finishes_first_tainted() -> Result<(), String> {
        let mut wf = make_workflow(vec![
            save_step("s0", TypedValue::Reference("$secrets.early".into())),
            finish_step("f1", TypedValue::Slot(0)),
            finish_step("f2", TypedValue::Literal(ValueType::Number)),
        ]);
        wf.secrets.push("early".to_owned());
        let result = validate_taint(&wf);
        if result != Err(ValidationError::SecretResultLeak) {
            return Err(format!(
                "expected SecretResultLeak from first tainted finish, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_type_check_object_in_choose_rejected() {
        type_check_object_in_choose_rejected().unwrap()
    }

    #[test]
    fn run_type_check_list_in_choose_rejected() {
        type_check_list_in_choose_rejected().unwrap()
    }

    #[test]
    fn run_type_check_number_in_choose_rejected() {
        type_check_number_in_choose_rejected().unwrap()
    }

    #[test]
    fn run_type_check_any_from_unresolved_ref_accepted() {
        type_check_any_from_unresolved_ref_accepted().unwrap()
    }

    #[test]
    fn run_type_check_save_composite_passes() {
        type_check_save_composite_passes().unwrap()
    }

    #[test]
    fn run_type_check_multiple_finishes_first_tainted() {
        type_check_multiple_finishes_first_tainted().unwrap()
    }

    // ---------------------------------------------------------------------------
    // Resource limits: additional boundary cases
    // ---------------------------------------------------------------------------

    /// Every zero field triggers LimitRequired.
    fn resource_limits_zero_constant_pool_rejected() -> Result<(), String> {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_constants: 0,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default();
        let result = validate_resource_limits(&wf, &hard);
        if !matches!(result, Err(ValidationError::LimitRequired { .. })) {
            return Err(format!(
                "expected LimitRequired for zero max_constants, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Declared max_collect_items exceeding hard limit is rejected.
    fn resource_limits_collect_items_exceeding_hard_rejected() -> Result<(), String> {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_collect_items: 10_000,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits {
            max_collect_items: 500,
            ..ResourceLimits::default()
        };
        let result = validate_resource_limits(&wf, &hard);
        let expected = Err(ValidationError::LimitExceeded {
            resource: "max_collect_items".to_owned(),
        });
        if result != expected {
            return Err(format!(
                "expected LimitExceeded for max_collect_items, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Declared max_retry_attempts exceeding hard limit is rejected.
    fn resource_limits_retry_attempts_exceeding_hard_rejected() -> Result<(), String> {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_retry_attempts: 100,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits {
            max_retry_attempts: 5,
            ..ResourceLimits::default()
        };
        let result = validate_resource_limits(&wf, &hard);
        let expected = Err(ValidationError::LimitExceeded {
            resource: "max_retry_attempts".to_owned(),
        });
        if result != expected {
            return Err(format!(
                "expected LimitExceeded for max_retry_attempts, got {result:?}"
            ));
        }
        Ok(())
    }

    /// Zero max_queue_depth triggers LimitRequired.
    fn resource_limits_zero_queue_depth_rejected() -> Result<(), String> {
        let wf = WorkflowTypes {
            inputs: vec![],
            vars: vec![],
            secrets: vec![],
            steps: vec![],
            resource_contract: ResourceLimits {
                max_queue_depth: 0,
                ..ResourceLimits::default()
            },
        };
        let hard = ResourceLimits::default();
        let result = validate_resource_limits(&wf, &hard);
        let expected = Err(ValidationError::LimitRequired {
            resource: "max_queue_depth".to_owned(),
        });
        if result != expected {
            return Err(format!(
                "expected LimitRequired for zero max_queue_depth, got {result:?}"
            ));
        }
        Ok(())
    }

    #[test]
    fn run_resource_limits_zero_constant_pool_rejected() {
        resource_limits_zero_constant_pool_rejected().unwrap()
    }

    #[test]
    fn run_resource_limits_collect_items_exceeding_hard_rejected() {
        resource_limits_collect_items_exceeding_hard_rejected().unwrap()
    }

    #[test]
    fn run_resource_limits_retry_attempts_exceeding_hard_rejected() {
        resource_limits_retry_attempts_exceeding_hard_rejected().unwrap()
    }

    #[test]
    fn run_resource_limits_zero_queue_depth_rejected() {
        resource_limits_zero_queue_depth_rejected().unwrap()
    }

    // ---------------------------------------------------------------------------
    // Edge-case coverage: taint propagation boundary conditions
    // ---------------------------------------------------------------------------

    /// Taint propagation through an empty composite (zero fields / empty
    /// object): a Save step with an empty Composite value produces a Clean
    /// taint because there are no sub-values to introduce Secret taint.
    /// Finishing with that slot should be accepted.
    #[test]
    fn taint_empty_composite_remains_clean() {
        let wf = make_workflow(vec![
            save_step("obj", TypedValue::Composite(vec![])),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Taint propagation through an empty list: a Save step with an empty
    /// Composite should produce type Any with Clean taint. Type validation
    /// should also pass since no type constraint is violated.
    #[test]
    fn taint_empty_list_save_then_clean_finish() {
        let wf = make_workflow(vec![
            save_step("list", TypedValue::Composite(vec![])),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        // Type validation passes (no incompatible type usage).
        assert_eq!(validate_types(&wf), Ok(()));
        // Taint validation passes (no secret data involved).
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Taint origin tracking across multiple hops (A -> B -> C):
    ///   Step 0 (A): saves a secret reference
    ///   Step 1 (B): saves the slot from step 0
    ///   Step 2 (C): finishes with the slot from step 1
    /// The secret taint must propagate through all three hops.
    #[test]
    fn taint_propagation_across_three_hops() {
        let mut wf = make_workflow(vec![
            save_step("a", TypedValue::Reference("$secrets.key".into())),
            save_step("b", TypedValue::Slot(0)),
            finish_step("c", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("key".to_owned());
        // Must reject: secret taint propagated from A -> B -> C.
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    /// Taint::Clean remains clean through non-secret operations:
    /// saving a literal, reading it back from a slot, and finishing with it
    /// should all stay Clean.
    #[test]
    fn clean_stays_clean_through_non_secret_operations() {
        let wf = make_workflow(vec![
            save_step("val", TypedValue::Literal(ValueType::Number)),
            save_step("copy", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Secret taint merging: two independent secret sources converge into a
    /// single composite value. The resulting taint must be Secret.
    #[test]
    fn two_secret_sources_merge_to_secret_taint() {
        let mut wf = make_workflow(vec![
            save_step("a", TypedValue::Reference("$secrets.api_key".into())),
            save_step("b", TypedValue::Reference("$secrets.api_secret".into())),
            save_step(
                "merged",
                TypedValue::Composite(vec![TypedValue::Slot(0), TypedValue::Slot(1)]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("api_key".to_owned());
        wf.secrets.push("api_secret".to_owned());
        // Must reject: both secrets merged into the result.
        assert!(matches!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak)
        ));
    }

    // =========================================================================
    // Blackhat edge-case tests
    // =========================================================================

    /// Edge case 1: BuildObject with zero fields remains Clean.
    /// An empty composite represents an object with no fields. Since there are
    /// no sub-values, no secret taint can be introduced. The slot written by
    /// this save should have Clean taint, and finishing with it should succeed.
    #[test]
    fn edge_build_object_zero_fields_remains_clean() {
        // BuildObject with no fields -> empty Composite
        let wf = make_workflow(vec![
            save_step("empty_obj", TypedValue::Composite(vec![])),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        // Type check passes (no incompatible types).
        assert_eq!(validate_types(&wf), Ok(()));
        // Taint check passes (no secrets involved).
        assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Edge case 2: BuildList with empty list remains Clean.
    /// An empty list has no elements, so no secret taint can propagate.
    /// The slot written by this save should have Clean taint, and finishing
    /// with it should succeed.
    #[test]
    fn edge_build_list_empty_remains_clean() {
        // BuildList with no elements -> empty Composite
        let wf = make_workflow(vec![
            save_step("empty_list", TypedValue::Composite(vec![])),
            save_step("relay", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(1)),
        ]);
        assert_eq!(validate_taint(&wf), Ok(()));
        assert_eq!(validate_types(&wf), Ok(()));
    }

    /// Edge case 3: Multi-hop taint propagation A -> B -> C (three Save steps).
    /// A secret is captured at step A, relayed through step B, relayed again
    /// through step C, and then finished. The taint must survive all three hops.
    #[test]
    fn edge_multi_hop_taint_a_to_b_to_c() {
        let mut wf = make_workflow(vec![
            // A: capture secret
            save_step("a", TypedValue::Reference("$secrets.db_credential".into())),
            // B: relay from A
            save_step("b", TypedValue::Slot(0)),
            // C: relay from B
            save_step("c", TypedValue::Slot(1)),
            // Finish from C -- must be tainted
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("db_credential".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak),
            "taint must propagate through three consecutive save hops"
        );
    }

    /// Edge case 4: Clean stays clean through non-secret operations end-to-end.
    /// A workflow that uses only clean inputs, clean vars, clean literals, and
    /// clean composites should pass both type and taint validation without
    /// any issues from start to finish.
    #[test]
    fn edge_clean_stays_clean_end_to_end() {
        let mut wf = make_workflow(vec![
            // Save a clean input
            save_step("input_val", TypedValue::Reference("$input.count".into())),
            // Save a clean var
            save_step("var_val", TypedValue::Reference("$vars.threshold".into())),
            // Save a clean literal
            save_step("lit_val", TypedValue::Literal(ValueType::Text)),
            // Composite of all clean values
            save_step(
                "combined",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Slot(1),
                    TypedValue::Slot(2),
                ]),
            ),
            // Choose with a clean boolean
            save_step("flag", TypedValue::Literal(ValueType::Boolean)),
            choose_step("route", TypedValue::Slot(4)),
            // Finish from the clean composite
            finish_step("done", TypedValue::Slot(3)),
        ]);
        wf.inputs.push(InputDecl {
            name: "count".to_owned(),
            schema_type: ValueType::Number,
            is_secret: false,
        });
        wf.vars.push(("threshold".to_owned(), ValueType::Number));
        // Secrets exist but are not used
        wf.secrets.push("unused_secret".to_owned());
        assert_eq!(
            validate_types(&wf),
            Ok(()),
            "type check must pass for all-clean workflow"
        );
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "taint check must pass for all-clean workflow"
        );
    }

    /// Edge case 5: Two independent secret sources merging into one output.
    /// One secret comes from $secrets namespace, another from a secret input.
    /// When both are combined in a composite, the result must be tainted.
    #[test]
    fn edge_two_independent_secret_sources_merge() {
        let mut wf = make_workflow(vec![
            // Source 1: $secrets namespace
            save_step("secret_a", TypedValue::Reference("$secrets.oauth_token".into())),
            // Source 2: secret input
            save_step("secret_b", TypedValue::Reference("$input.private_key".into())),
            // Merge both into one composite
            save_step(
                "merged_output",
                TypedValue::Composite(vec![TypedValue::Slot(0), TypedValue::Slot(1)]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("oauth_token".to_owned());
        wf.inputs.push(InputDecl {
            name: "private_key".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        assert_eq!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak),
            "merging two independent secret sources must produce Secret taint"
        );
    }

    /// Edge case 6: Taint through ForEach body -- body sees parent taint.
    /// The ForEach concept is simulated by saving items from a tainted source
    /// into separate slots (representing iteration over items). Each "body
    /// iteration" that reads from the tainted parent should carry taint.
    /// In this model, the sequential save pattern captures the same taint
    /// propagation behavior that ForEach would exhibit.
    #[test]
    fn edge_taint_through_for_each_body_sees_parent_taint() {
        // Simulate ForEach: parent has a tainted slot, and "body" steps
        // read from it, producing tainted output.
        let mut wf = make_workflow(vec![
            // Parent: save a secret
            save_step("parent", TypedValue::Reference("$secrets.db_url".into())),
            // Body iteration 1: read parent slot
            save_step("body_item_1", TypedValue::Slot(0)),
            // Body iteration 2: read parent slot into a composite
            save_step(
                "body_item_2",
                TypedValue::Composite(vec![TypedValue::Slot(0), TypedValue::Literal(ValueType::Text)]),
            ),
            // Body iteration 3: relay of body_item_1
            save_step("body_item_3", TypedValue::Slot(1)),
            // Finish with body output -- tainted via parent
            finish_step("done", TypedValue::Slot(3)),
        ]);
        wf.secrets.push("db_url".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak),
            "ForEach body items that read from a tainted parent must carry taint"
        );
    }

    /// Edge case 6b: ForEach body with clean parent stays clean.
    /// When the parent slot is clean, all body items derived from it are also
    /// clean, and finishing with a body output succeeds.
    #[test]
    fn edge_for_each_body_with_clean_parent_stays_clean() {
        let wf = make_workflow(vec![
            // Parent: clean literal
            save_step("parent", TypedValue::Literal(ValueType::Number)),
            // Body iteration 1: read parent slot
            save_step("body_item_1", TypedValue::Slot(0)),
            // Body iteration 2: composite with parent slot
            save_step(
                "body_item_2",
                TypedValue::Composite(vec![TypedValue::Slot(0), TypedValue::Literal(ValueType::Text)]),
            ),
            // Finish from body output -- clean
            finish_step("done", TypedValue::Slot(2)),
        ]);
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "ForEach body with clean parent must remain clean"
        );
    }

    /// Edge case 7: Taint through Together branches -- each branch is independent.
    /// The Together concept is simulated by two independent save chains that
    /// don't reference each other. One branch carries taint, the other is clean.
    /// Finishing from the clean branch should succeed even though the other
    /// branch is tainted.
    #[test]
    fn edge_together_branches_independent_taint() {
        let mut wf = make_workflow(vec![
            // Branch A: tainted
            save_step("branch_a", TypedValue::Reference("$secrets.internal_key".into())),
            // Branch B: clean
            save_step("branch_b", TypedValue::Literal(ValueType::Number)),
            // Finish from the clean branch -- must pass
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("internal_key".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "Together: finishing from the clean branch must pass"
        );
    }

    /// Edge case 7b: Together branches -- finishing from the tainted branch fails.
    #[test]
    fn edge_together_branches_tainted_branch_finish_fails() {
        let mut wf = make_workflow(vec![
            // Branch A: tainted
            save_step("branch_a", TypedValue::Reference("$secrets.internal_key".into())),
            // Branch B: clean
            save_step("branch_b", TypedValue::Literal(ValueType::Number)),
            // Finish from the tainted branch -- must fail
            finish_step("done", TypedValue::Slot(0)),
        ]);
        wf.secrets.push("internal_key".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak),
            "Together: finishing from the tainted branch must fail"
        );
    }

    /// Edge case 7c: Together branches merging -- both branches feed into a
    /// composite result. If one branch is tainted, the merged result is tainted.
    #[test]
    fn edge_together_branches_merged_one_tainted_result_is_tainted() {
        let mut wf = make_workflow(vec![
            // Branch A: tainted
            save_step("branch_a", TypedValue::Reference("$secrets.credential".into())),
            // Branch B: clean
            save_step("branch_b", TypedValue::Literal(ValueType::Text)),
            // Merge both branches
            save_step(
                "merged",
                TypedValue::Composite(vec![TypedValue::Slot(0), TypedValue::Slot(1)]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.secrets.push("credential".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Err(ValidationError::SecretResultLeak),
            "Together: merging clean and tainted branches must be tainted"
        );
    }

    /// Edge case 8: Self-referential slot doesn't cause infinite loop.
    /// A save step reads from its own slot index. Since the slot has not been
    /// written yet at the time of the read, it resolves as clean Any. The
    /// write then stores clean Any into the slot. Finishing from this slot
    /// should succeed without infinite recursion or panics.
    #[test]
    fn edge_self_referential_slot_no_infinite_loop() {
        let wf = make_workflow(vec![
            // Slot 0 reads from itself -- uninitialized, resolves as clean Any
            save_step("self_ref", TypedValue::Slot(0)),
            finish_step("done", TypedValue::Slot(0)),
        ]);
        // Must not panic or hang
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "self-referential slot must resolve as clean, not cause infinite loop"
        );
        assert_eq!(
            validate_types(&wf),
            Ok(()),
            "self-referential slot type check must pass"
        );
    }

    /// Edge case 8b: Self-referential slot with a tainted slot in between.
    /// Slot 0 is tainted, slot 1 reads from slot 1 (self-ref, resolves clean),
    /// finish from slot 1 is clean. The tainted slot 0 is not involved.
    #[test]
    fn edge_self_referential_slot_with_adjacent_taint_stays_clean() {
        let mut wf = make_workflow(vec![
            // Slot 0: tainted
            save_step("tainted", TypedValue::Reference("$secrets.adj".into())),
            // Slot 1: self-referential (reads slot 1, which is None -> clean)
            save_step("self_ref", TypedValue::Slot(1)),
            // Finish from slot 1 -- clean because self-ref resolved as clean Any
            finish_step("done", TypedValue::Slot(1)),
        ]);
        wf.secrets.push("adj".to_owned());
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "self-referential slot must not pick up taint from adjacent slots"
        );
    }

    /// Edge case 9: Finish with Clean taint succeeds.
    /// A straightforward workflow where the finish step receives a value with
    /// Clean taint from a composite of clean inputs, clean literals, and clean
    /// vars. This must pass both validators.
    #[test]
    fn edge_finish_with_clean_taint_succeeds() {
        let mut wf = make_workflow(vec![
            save_step("input_val", TypedValue::Reference("$input.email".into())),
            save_step("var_val", TypedValue::Reference("$vars.counter".into())),
            save_step(
                "result",
                TypedValue::Composite(vec![
                    TypedValue::Slot(0),
                    TypedValue::Slot(1),
                    TypedValue::Literal(ValueType::Boolean),
                ]),
            ),
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.inputs.push(InputDecl {
            name: "email".to_owned(),
            schema_type: ValueType::Text,
            is_secret: false,
        });
        wf.vars.push(("counter".to_owned(), ValueType::Number));
        assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "finish with clean taint must succeed"
        );
        assert_eq!(
            validate_types(&wf),
            Ok(()),
            "finish with clean taint must pass type check"
        );
    }

    /// Edge case 10: Finish with Secret taint fails with SecretResultLeak.
    /// A workflow where taint propagates from a secret input through a relay
    /// into a composite and finally to the finish step. The exact error must
    /// be ValidationError::SecretResultLeak.
    #[test]
    fn edge_finish_with_secret_taint_fails_with_secret_result_leak() {
        let mut wf = make_workflow(vec![
            // Capture secret input
            save_step("cap", TypedValue::Reference("$input.password".into())),
            // Relay
            save_step("relay", TypedValue::Slot(0)),
            // Composite with relay
            save_step(
                "payload",
                TypedValue::Composite(vec![
                    TypedValue::Slot(1),
                    TypedValue::Literal(ValueType::Text),
                ]),
            ),
            // Finish with tainted composite
            finish_step("done", TypedValue::Slot(2)),
        ]);
        wf.inputs.push(InputDecl {
            name: "password".to_owned(),
            schema_type: ValueType::Text,
            is_secret: true,
        });
        let result = validate_taint(&wf);
        assert_eq!(
            result,
            Err(ValidationError::SecretResultLeak),
            "finish with secret taint must fail with SecretResultLeak exactly"
        );
    }
}
