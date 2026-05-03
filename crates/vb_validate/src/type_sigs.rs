//! Type primitives for type/taint validation.

#![allow(unreachable_pub)]
//!
//! Defines ValueType, Taint, ValueFact, TypedValue, and workflow model types
//! used to resolve references and track type+taint through workflow steps.

// ---------------------------------------------------------------------------
// Value type primitives
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
// Workflow model types
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
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    // -- ValueType::as_str tests --

    #[test]
    fn value_type_as_str_returns_correct_names() {
        assert_eq!(ValueType::Null.as_str(), "null");
        assert_eq!(ValueType::Boolean.as_str(), "boolean");
        assert_eq!(ValueType::Number.as_str(), "number");
        assert_eq!(ValueType::Text.as_str(), "text");
        assert_eq!(ValueType::Object.as_str(), "object");
        assert_eq!(ValueType::List.as_str(), "list");
        assert_eq!(ValueType::Any.as_str(), "any");
    }

    // -- Taint::merge tests --

    #[test]
    fn taint_merge_clean_clean_is_clean() {
        assert_eq!(Taint::Clean.merge(Taint::Clean), Taint::Clean);
    }

    #[test]
    fn taint_merge_secret_clean_is_secret() {
        assert_eq!(Taint::Secret.merge(Taint::Clean), Taint::Secret);
    }

    #[test]
    fn taint_merge_clean_secret_is_secret() {
        assert_eq!(Taint::Clean.merge(Taint::Secret), Taint::Secret);
    }

    #[test]
    fn taint_merge_secret_secret_is_secret() {
        assert_eq!(Taint::Secret.merge(Taint::Secret), Taint::Secret);
    }

    // -- ValueFact constructor tests --

    #[test]
    fn value_fact_clean_creates_clean_taint() {
        let fact = ValueFact::clean(ValueType::Number);
        assert_eq!(fact.value_type, ValueType::Number);
        assert_eq!(fact.taint, Taint::Clean);
    }

    #[test]
    fn value_fact_secret_creates_secret_taint() {
        let fact = ValueFact::secret(ValueType::Text);
        assert_eq!(fact.value_type, ValueType::Text);
        assert_eq!(fact.taint, Taint::Secret);
    }

    #[test]
    fn value_fact_clean_all_types() {
        let types = [
            ValueType::Null,
            ValueType::Boolean,
            ValueType::Number,
            ValueType::Text,
            ValueType::Object,
            ValueType::List,
            ValueType::Any,
        ];
        for vt in types {
            let fact = ValueFact::clean(vt);
            assert_eq!(fact.value_type, vt);
            assert_eq!(fact.taint, Taint::Clean);
        }
    }

    #[test]
    fn value_fact_secret_all_types() {
        let types = [
            ValueType::Null,
            ValueType::Boolean,
            ValueType::Number,
            ValueType::Text,
            ValueType::Object,
            ValueType::List,
            ValueType::Any,
        ];
        for vt in types {
            let fact = ValueFact::secret(vt);
            assert_eq!(fact.value_type, vt);
            assert_eq!(fact.taint, Taint::Secret);
        }
    }

    // -- ResourceLimits default tests --

    #[test]
    fn resource_limits_default_values() {
        let limits = ResourceLimits::default();
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

    #[test]
    fn workflow_types_default_is_empty() {
        let wf = WorkflowTypes::default();
        assert!(wf.inputs.is_empty());
        assert!(wf.vars.is_empty());
        assert!(wf.secrets.is_empty());
        assert!(wf.steps.is_empty());
    }

    // -- ValueType Copy/PartialEq tests --

    #[test]
    fn value_type_equality() {
        assert_eq!(ValueType::Number, ValueType::Number);
        assert_ne!(ValueType::Number, ValueType::Text);
    }

    // -- Taint Copy/PartialEq tests --

    #[test]
    fn taint_equality() {
        assert_eq!(Taint::Clean, Taint::Clean);
        assert_eq!(Taint::Secret, Taint::Secret);
        assert_ne!(Taint::Clean, Taint::Secret);
    }
}
