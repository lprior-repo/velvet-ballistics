#![forbid(unsafe_code)]
//! Trigger, authoring value, and step AST data.

/// Trigger declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TriggerAst {
    /// Manual trigger (default).
    Manual,
    /// Schedule trigger with cron expression.
    Schedule { cron: String },
    /// Named event trigger; YAML field is `name`.
    Event { event_type: String },
    /// Empty webhook trigger.
    Webhook,
}

/// Recursive cold authoring value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthorValue {
    Null,
    Bool(bool),
    I64(i64),
    Text(String),
    Sequence(Vec<AuthorValue>),
    Mapping(Vec<AuthorEntry<AuthorValue>>),
}

/// Key/value entry used for author mappings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorEntry<T> {
    pub key: String,
    pub value: T,
}

/// A single workflow step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAst {
    /// Unique step identifier.
    pub id: String,
    /// Human-readable name (optional).
    pub name: Option<String>,
    /// Condition expression for conditional execution.
    pub condition: Option<String>,
    /// The primitive operation.
    pub primitive: StepPrimitive,
    /// Resource / connector reference (optional).
    pub with: Option<String>,
    /// Retry policy (optional).
    pub retry: Option<RetryPolicy>,
    /// Error handler (optional).
    pub on_error: Option<ErrorHandlerAst>,
    /// Next-step label for explicit flow control (optional).
    pub then: Option<String>,
}

/// The concrete primitive operation for a step.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StepPrimitive {
    /// Set a variable to a value.
    Set {
        /// Target variable name.
        output: String,
        /// Value expression.
        value: String,
    },
    /// Save a constant value to slots (compile-layer alias for set).
    Save {
        /// Constant value expression.
        value: ScalarValue,
    },
    /// Execute an action.
    Do {
        /// Action identifier.
        action: String,
        /// Input expression.
        input: String,
    },
    /// Branching construct.
    Choose {
        /// Branch list.
        branches: Vec<ChooseBranch>,
        /// Default branch label (optional).
        otherwise: Option<String>,
    },
    /// Parallel fan-out.
    ForEach {
        /// Loop variable name.
        variable: String,
        /// Input collection expression.
        input: String,
        /// Maximum concurrency (optional).
        at_once: Option<u32>,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Concurrent branches that run together.
    Together {
        /// Branch list.
        branches: Vec<TogetherBranch>,
    },
    /// Paginated collection loop.
    Collect {
        /// Loop variable name.
        variable: String,
        /// Source expression.
        source: String,
        /// Maximum pages (optional).
        pages: Option<u32>,
        /// Items per page (optional).
        items: Option<u32>,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Left-fold aggregation.
    Aggregate {
        /// Accumulator variable name.
        variable: String,
        /// Input collection expression.
        input: String,
        /// Initial value expression.
        initial: String,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Retry loop.
    Repeat {
        /// Maximum retry attempts.
        max_attempts: u16,
        /// Body steps.
        body: Vec<StepAst>,
    },
    /// Wait for an event or timeout.
    Wait {
        /// Event expression (optional).
        event: Option<String>,
        /// Timeout expression (optional).
        timeout: Option<String>,
    },
    /// Ask for human input.
    Ask {
        /// Prompt text.
        prompt: String,
        /// Timeout expression (optional).
        timeout: Option<String>,
    },
    /// Terminate the workflow with a result.
    Finish {
        /// Result expression or literal.
        result: ScalarValue,
    },
}

/// A scalar YAML value used in step fields.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScalarValue {
    /// A string value.
    String(String),
    /// An integer value.
    Integer(i64),
}

/// A branch inside a `Choose` primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChooseBranch {
    /// Condition label (the "when" field).
    pub when: String,
    /// Steps to execute when the condition matches.
    pub steps: Vec<StepAst>,
}

/// A branch inside a `Together` primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TogetherBranch {
    /// Branch label.
    pub label: String,
    /// Steps to execute in this branch.
    pub steps: Vec<StepAst>,
}

/// Retry policy for a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum retry attempts.
    pub max_attempts: u16,
    /// Delay between retries (expression or duration string).
    pub delay: Option<String>,
}

/// Error handler attached to a step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorHandlerAst {
    /// Handler label or step reference.
    pub handler: String,
}
