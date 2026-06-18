#![forbid(unsafe_code)]
//! Workflow model types for type and taint validation.
//!
//! Declares the input model structs that consumers populate before passing
//! a workflow to the validators in the parent module.

// ---------------------------------------------------------------------------
// Workflow model
// ---------------------------------------------------------------------------

/// Input schema type declaration.
#[derive(Debug, Clone)]
pub struct InputDecl {
    /// Input name.
    pub name: String,
    /// Declared type.
    pub schema_type: super::types::ValueType,
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
    /// Whether the workflow is allowed to produce secret-tainted results.
    pub allows_secret_results: bool,
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
            allows_secret_results: false,
        }
    }
}

/// Workflow model for type/taint validation.
#[derive(Debug, Clone, Default)]
pub struct WorkflowTypes {
    /// Declared inputs with their schemas.
    pub inputs: Vec<InputDecl>,
    /// Declared vars with their types.
    pub vars: Vec<(String, super::types::ValueType)>,
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
#[non_exhaustive]
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
#[non_exhaustive]
pub enum TypedValue {
    /// A literal with known type.
    Literal(super::types::ValueType),
    /// A reference to a declared name (e.g., `$input.user`).
    Reference(String),
    /// A slot reference by step index.
    Slot(usize),
    /// A composite value (list/object) with sub-values.
    Composite(Vec<TypedValue>),
}
