#![forbid(unsafe_code)]
use crate::SourceMark;
use crate::expression::ParsedExpression;
use vb_core::{ActionId, SlotIdx, StepIdx};

/// A parsed workflow document before numeric IR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowAst {
    /// Language version string from `version`.
    pub version: Box<str>,
    /// Public workflow name.
    pub name: Box<str>,
    /// Single workflow trigger.
    pub trigger: TriggerAst,
    /// Input declarations retained as cold values.
    pub inputs: Vec<AstMapEntry<AstValue>>,
    /// Static variable declarations retained as cold values.
    pub vars: Vec<AstMapEntry<AstValue>>,
    /// Secret requirement bindings.
    pub secrets: Vec<AstMapEntry<Box<str>>>,
    /// Top-level result mapping expressions.
    pub result: Vec<AstMapEntry<AstExpression>>,
    /// Example documents retained as cold values.
    pub examples: Vec<AstValue>,
    /// Source steps in declaration order.
    pub steps: Vec<StepAst>,
    /// Best available document source mark.
    pub mark: Option<SourceMark>,
}

/// A named AST map entry with an optional source mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstMapEntry<T> {
    /// Mapping key.
    pub name: Box<str>,
    /// Parsed mapping value.
    pub value: T,
    /// Best available key source mark.
    pub mark: Option<SourceMark>,
}

/// Supported trigger forms in the cold compiler-side AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TriggerAst {
    /// Direct/manual Rust API submission.
    Manual {
        /// Source mark for the `manual` trigger key.
        mark: Option<SourceMark>,
    },
    /// Cold adapter webhook trigger retained for compiler compatibility.
    Webhook {
        /// Webhook path (optional for adapter flexibility).
        path: Option<Box<str>>,
        /// HTTP method retained for cold adapter compilation.
        method: Option<Box<str>>,
        /// Optional idempotency expression/source field.
        unique: Option<Box<str>>,
        /// Source mark for the `webhook` trigger key.
        mark: Option<SourceMark>,
    },
    /// Cron-like schedule trigger retained for compiler compatibility.
    Schedule {
        /// Five-field cron expression.
        cron: Box<str>,
        /// Optional timezone name.
        timezone: Option<Box<str>>,
        /// Source mark for the `schedule` trigger key.
        mark: Option<SourceMark>,
    },
    /// Named event trigger retained for compiler compatibility.
    Event {
        /// Event name.
        name: Box<str>,
        /// Source mark for the `event` trigger key.
        mark: Option<SourceMark>,
    },
}

/// One high-level source step before IR expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAst {
    /// Required step ID.
    pub id: Box<str>,
    /// Optional display name.
    pub name: Option<Box<str>>,
    /// Exact source primitive spelling before semantic lowering.
    pub primitive: StepPrimitiveAst,
    /// Parsed high-level primitive.
    pub kind: StepKindAst,
    /// Best available step source mark.
    pub mark: Option<SourceMark>,
}

/// Exact high-level primitive spelling retained from the source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StepPrimitiveAst {
    /// `set` constant-producing primitive.
    Set,
    /// `run` action boundary primitive.
    Run,
    /// `do` action boundary primitive alias.
    Do,
    /// `save` legacy constant-producing primitive.
    Save,
    /// `choose` branch primitive.
    Choose,
    /// `for_each` bounded iteration primitive.
    ForEach,
    /// `parallel` bounded fanout primitive.
    Parallel,
    /// `collect` bounded collection primitive.
    Collect,
    /// `aggregate` bounded reduction primitive.
    Aggregate,
    /// `repeat` bounded repeat primitive.
    Repeat,
    /// `wait` boundary primitive.
    Wait,
    /// `ask` boundary primitive.
    Ask,
    /// `finish` terminal primitive.
    Finish,
}

/// High-level primitive intent recognized by the compiler AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StepKindAst {
    /// Numeric action boundary primitive.
    Run {
        /// Resolved action identifier.
        action: ActionId,
        /// Input slot supplied to the action.
        input: SlotIdx,
    },
    /// Constant-producing `save` primitive.
    Save {
        /// Output fields declared by the save body.
        fields: Vec<AstMapEntry<AstValue>>,
    },
    /// Boolean branch primitive.
    Choose {
        /// Branch condition expression.
        condition: AstExpression,
        /// Target step when the condition is true.
        on_true: StepIdx,
        /// Target step when the condition is false.
        on_false: StepIdx,
    },
    /// Low-level bounded `for_each` primitive.
    ForEach {
        input: SlotIdx,
        item: SlotIdx,
        limit: u32,
    },
    /// Low-level bounded `together` primitive.
    Together { branches: Vec<StepIdx> },
    /// Low-level bounded `collect` primitive.
    Collect {
        source: SlotIdx,
        limit: u32,
        page_size: u32,
    },
    /// Low-level bounded `reduce` primitive.
    Reduce {
        input: SlotIdx,
        accumulator: SlotIdx,
        initial: AstValue,
    },
    /// Low-level bounded `repeat` primitive.
    Repeat { max_attempts: u16 },
    /// Wait boundary primitive.
    Wait {
        /// Deadline or event slot.
        slot: SlotIdx,
        /// Optional timeout slot for event waits.
        timeout: Option<SlotIdx>,
        /// Whether the slot names an event instead of a deadline.
        is_event: bool,
    },
    /// Ask boundary primitive.
    Ask {
        /// Prompt slot supplied to the asker.
        prompt: SlotIdx,
        /// Answer slot filled on resume.
        answer: SlotIdx,
        /// Optional timeout slot.
        timeout: Option<SlotIdx>,
    },
    /// Finish primitive.
    Finish {
        /// Final result expression.
        result: AstExpression,
    },
}

/// Literal tree retained in the cold AST.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AstValue {
    /// YAML null.
    Null,
    /// Boolean scalar.
    Bool(bool),
    /// Integer scalar.
    I64(i64),
    /// Text scalar.
    Text(Box<str>),
    /// Reference string beginning with `$`.
    Reference(Box<str>),
    /// Sequence value.
    Sequence(Vec<AstValue>),
    /// Mapping value.
    Mapping(Vec<AstMapEntry<AstValue>>),
}

/// Expression/reference placeholder before bytecode lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AstExpression {
    /// Numeric slot reference retained from the Phase 0 grammar.
    Slot(SlotIdx),
    /// Source reference string beginning with `$`.
    Reference(Box<str>),
    /// Parsed v1 source expression retained for later bytecode lowering.
    Parsed(Box<ParsedExpression>),
    /// Literal expression.
    Literal(AstValue),
}
