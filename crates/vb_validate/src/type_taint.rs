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
}
