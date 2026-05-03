fn non_string_key_error() -> CompileError {
    CompileError::NonStringKey {
        mark: SourceMark::unavailable(),
    }
}

/// YAML compiler errors.
#[derive(Debug, Clone, Error)]
pub enum CompileError {
    /// Source exceeded configured byte limit.
    #[error("YAML source exceeds byte limit: actual={actual}, limit={limit}")]
    SourceTooLarge {
        /// Actual source size.
        actual: usize,
        /// Configured limit.
        limit: usize,
    },
    /// Source was not UTF-8.
    #[error("YAML source must be UTF-8: {0}")]
    Utf8(#[from] str::Utf8Error),
    /// Source did not contain a YAML document.
    #[error("YAML source must contain exactly one non-empty document")]
    EmptySource,
    /// Native YAML parser rejected the document.
    #[error("YAML parse failed: {0}")]
    Parse(#[from] saphyr::ScanError),
    /// YAML streams are forbidden.
    #[error("expected exactly one YAML document, found {count}")]
    DocumentCount {
        /// Document count found by parser.
        count: usize,
    },
    /// The top-level YAML node must be a mapping.
    #[error("top-level YAML document must be a mapping")]
    TopLevelNotMapping,
    /// Mapping keys must be strings.
    #[error("mapping key must be a string at {mark:?}")]
    NonStringKey {
        /// Best available source mark.
        mark: SourceMark,
    },
    /// YAML mappings must not contain duplicate keys.
    #[error("duplicate YAML mapping key: {key} at {mark:?}")]
    DuplicateKey {
        /// Duplicated key.
        key: Box<str>,
        /// Best available source mark.
        mark: SourceMark,
    },
    /// YAML anchors/aliases are forbidden.
    #[error("YAML aliases are forbidden at {mark:?}")]
    AliasForbidden {
        /// Parser mark for the alias event.
        mark: SourceMark,
    },
    /// YAML anchors are forbidden.
    #[error("YAML anchors are forbidden at {mark:?}")]
    AnchorForbidden {
        /// Parser mark for the anchored node.
        mark: SourceMark,
    },
    /// YAML merge keys are forbidden.
    #[error("YAML merge keys are forbidden at {mark:?}")]
    MergeKeyForbidden {
        /// Best available source mark.
        mark: SourceMark,
    },
    /// YAML tags are forbidden.
    #[error("YAML tags are forbidden at {mark:?}")]
    TagForbidden {
        /// Parser mark for the tagged node.
        mark: SourceMark,
    },
    /// Saphyr produced a bad scalar value.
    #[error("YAML scalar value is invalid")]
    BadValue,
    /// Floating-point YAML scalars are forbidden in the initial profile.
    #[error("floating-point YAML scalars are forbidden")]
    FloatForbidden,
    /// YAML depth exceeded configured limit.
    #[error("YAML nesting depth exceeds limit: depth={depth}, limit={limit}")]
    DepthLimit {
        /// Observed depth.
        depth: u16,
        /// Configured depth limit.
        limit: u16,
    },
    /// YAML node count exceeded configured limit.
    #[error("YAML node count exceeds limit: limit={limit}")]
    NodeLimit {
        /// Configured node limit.
        limit: u32,
    },
    /// YAML sequence exceeded configured limit.
    #[error("YAML sequence length exceeds limit: actual={actual}, limit={limit}")]
    SequenceLimit {
        /// Actual sequence length.
        actual: usize,
        /// Configured sequence limit.
        limit: usize,
    },
    /// YAML mapping exceeded configured limit.
    #[error("YAML mapping entry count exceeds limit: actual={actual}, limit={limit}")]
    MappingLimit {
        /// Actual mapping entries.
        actual: usize,
        /// Configured mapping limit.
        limit: usize,
    },
    /// YAML scalar exceeded configured limit.
    #[error("YAML scalar length exceeds limit: actual={actual}, limit={limit}")]
    ScalarLimit {
        /// Actual scalar length.
        actual: usize,
        /// Configured scalar limit.
        limit: usize,
    },
    /// Compiled IR validation failed.
    #[error("compiled workflow IR failed validation: {0}")]
    Workflow(#[from] WorkflowError),
    /// Shared validation pipeline gate failure.
    #[error("validation gate failure: {0}")]
    Validation(#[from] vb_validate::ValidationError),
    /// Required workflow field is missing.
    #[error("required workflow field is missing: {field}")]
    MissingField {
        /// Missing field name.
        field: &'static str,
    },
    /// Top-level workflow field is not part of the supported schema.
    #[error("unknown top-level workflow field: {field}")]
    UnknownTopLevelField {
        /// Unknown field name.
        field: Box<str>,
    },
    /// Workflow version must match the public Velvet v1 version exactly.
    #[error("unsupported workflow version: {actual}")]
    InvalidVersion {
        /// Version found in source YAML.
        actual: Box<str>,
    },
    /// Workflow trigger declaration must contain exactly one trigger.
    #[error("workflow when must declare exactly one trigger, found {count}")]
    InvalidTriggerCount {
        /// Number of trigger entries found.
        count: usize,
    },
    /// Trigger kind is not part of Velvet v1.
    #[error("unknown workflow trigger kind: {trigger}")]
    UnknownTriggerKind {
        /// Unknown trigger kind.
        trigger: Box<str>,
    },
    /// Trigger configuration has the wrong YAML shape.
    #[error("trigger {trigger} must be {expected}")]
    TriggerShape {
        /// Trigger kind.
        trigger: Box<str>,
        /// Expected shape.
        expected: &'static str,
    },
    /// Trigger field is not valid for the selected trigger kind.
    #[error("trigger {trigger} has unknown field: {field}")]
    UnknownTriggerField {
        /// Trigger kind.
        trigger: &'static str,
        /// Unknown trigger field.
        field: Box<str>,
    },
    /// Required trigger field is missing.
    #[error("trigger {trigger} is missing required field: {field}")]
    MissingTriggerField {
        /// Trigger kind.
        trigger: &'static str,
        /// Missing trigger field.
        field: &'static str,
    },
    /// Trigger field value failed semantic validation.
    #[error("trigger {trigger} field {field} must be {expected}")]
    InvalidTriggerField {
        /// Trigger kind.
        trigger: &'static str,
        /// Trigger field.
        field: &'static str,
        /// Expected value shape or semantic rule.
        expected: &'static str,
    },
    /// Workflow field has the wrong YAML shape.
    #[error("workflow field {field} must be {expected}")]
    FieldShape {
        /// Field name.
        field: &'static str,
        /// Expected shape.
        expected: &'static str,
    },
    /// Input schema field is not part of Velvet v1.
    #[error("input schema has unknown field: {field}")]
    UnknownInputSchemaField {
        /// Unknown schema field.
        field: Box<str>,
    },
    /// Input schema field failed shape or semantic validation.
    #[error("input schema field {field} must be {expected}")]
    InvalidInputSchema {
        /// Schema field path.
        field: &'static str,
        /// Expected shape or semantic rule.
        expected: &'static str,
    },
    /// Phase 0 compiler does not yet compile top-level result mappings.
    #[error("non-empty top-level result is not supported by the Phase 0 compiler")]
    UnsupportedTopLevelResult,
    /// Workflow must contain at least one executable step.
    #[error("workflow steps must not be empty")]
    EmptySteps,
    /// Public workflow or step name does not match the Velvet v1 identifier grammar.
    #[error("{field} is not a valid Velvet v1 name: {value}")]
    InvalidName {
        /// Field containing the invalid name.
        field: &'static str,
        /// Invalid name value.
        value: Box<str>,
    },
    /// Step is missing its required public ID.
    #[error("step {step} is missing required id")]
    MissingStepId {
        /// Step index.
        step: usize,
    },
    /// Step ID appears more than once in the workflow.
    #[error("duplicate step id: {id}")]
    DuplicateStepId {
        /// Duplicate step ID.
        id: Box<str>,
    },
    /// Step must be a mapping.
    #[error("step {step} must be a mapping")]
    StepShape {
        /// Step index.
        step: usize,
    },
    /// Step field is not part of the Velvet v1 schema.
    #[error("step {step} has unknown field: {field}")]
    UnknownStepField {
        /// Step index.
        step: usize,
        /// Unknown field name.
        field: Box<str>,
    },
    /// Primitive body field is not accepted by the Phase 0 compiler.
    #[error("step {step} primitive {primitive} has unknown field: {field}")]
    UnknownStepPrimitiveField {
        /// Step index.
        step: usize,
        /// Primitive containing the field.
        primitive: &'static str,
        /// Unknown primitive field.
        field: Box<str>,
    },
    /// Step is missing its single required primitive.
    #[error("step {step} is missing a primitive field")]
    MissingStepPrimitive {
        /// Step index.
        step: usize,
    },
    /// Step contains more than one primitive.
    #[error("step {step} has multiple primitive fields")]
    MultipleStepPrimitives {
        /// Step index.
        step: usize,
    },
    /// Primitive is valid Velvet v1 but not compiled by the Phase 0 IR subset.
    #[error("step {step} primitive {primitive} is not supported by the Phase 0 compiler")]
    UnsupportedStepPrimitive {
        /// Step index.
        step: usize,
        /// Canonical primitive name.
        primitive: &'static str,
    },
    /// Step control field is valid Velvet v1 but not compiled by the Phase 0 IR subset.
    #[error("step {step} control field {field} is not supported by the Phase 0 compiler")]
    UnsupportedStepControlField {
        /// Step index.
        step: usize,
        /// Unsupported control field.
        field: Box<str>,
    },
    /// Required step field is missing.
    #[error("step {step} is missing required field: {field}")]
    MissingStepField {
        /// Step index.
        step: usize,
        /// Missing field name.
        field: &'static str,
    },
    /// Step field has the wrong YAML shape.
    #[error("step {step} field {field} must be {expected}")]
    StepFieldShape {
        /// Step index.
        step: usize,
        /// Field name.
        field: &'static str,
        /// Expected shape.
        expected: &'static str,
    },
    /// Numeric step index exceeds the IR representation.
    #[error("step index exceeds u16: {value}")]
    StepIndexOutOfRange {
        /// Invalid value.
        value: usize,
    },
    /// Slot index must be an unsigned u16.
    #[error("slot index is outside u16 range: {value}")]
    SlotIndexOutOfRange {
        /// Invalid value.
        value: i64,
    },
    /// Branch target must be an unsigned u16.
    #[error("branch target is outside u16 range: {value}")]
    BranchTargetOutOfRange {
        /// Invalid value.
        value: i64,
    },
    /// Branch target must point forward in v1.
    #[error("branch target {target} at step {step} must point forward")]
    BackwardBranchTarget {
        /// Step containing the branch.
        step: usize,
        /// Invalid target.
        target: usize,
    },
    /// Primitive lowering would exceed a bounded compiler representation.
    #[error("step primitive {primitive} field {field} value {value} exceeds limit {limit}")]
    PrimitiveLoweringLimitExceeded {
        /// Primitive being lowered.
        primitive: &'static str,
        /// Bounded field being computed.
        field: &'static str,
        /// Attempted value or source value at the limit.
        value: usize,
        /// Maximum accepted representation value.
        limit: usize,
    },
    /// Linear workflows must end with an explicit finish step.
    #[error("last workflow step must be finish")]
    LastStepMustFinish,
    /// Constant values must be scalar YAML values.
    #[error("constant value for step {step} must be a scalar")]
    UnsupportedConstantValue {
        /// Step index.
        step: usize,
    },
    /// Reference root is not part of the bounded Velvet v1 reference surface.
    #[error("unknown reference root in {reference}: {root}")]
    UnknownReferenceRoot {
        /// Full source reference string.
        reference: Box<str>,
        /// Unknown root segment without the leading `$`.
        root: Box<str>,
    },
    /// Reference root is known but forbidden in deterministic compiled IR.
    #[error("illegal reference in deterministic workflow: {reference}")]
    IllegalReference {
        /// Full source reference string.
        reference: Box<str>,
    },
    /// Reference points at an undeclared input, variable, secret, or step.
    #[error("unknown {kind} reference in {reference}: {name}")]
    UnknownReferenceName {
        /// Declaration table that was searched.
        kind: &'static str,
        /// Full source reference string.
        reference: Box<str>,
        /// Missing declaration name.
        name: Box<str>,
    },
    /// Reference uses an accessor path outside the current compiled surface.
    #[error("unsupported accessor reference in {reference}: {root}.{path}")]
    UnsupportedAccessorReference {
        /// Full source reference string.
        reference: Box<str>,
        /// Resolved root segment.
        root: Box<str>,
        /// Unsupported accessor tail.
        path: Box<str>,
    },
    /// Branch target points outside the declared step table.
    #[error("step {step} branch target {target} is not a declared step")]
    UnknownStepTarget {
        /// Step containing the invalid target.
        step: usize,
        /// Missing target index.
        target: usize,
    },
    /// A declared step cannot be reached from the entry step.
    #[error("step {step} is unreachable from workflow entry")]
    UnreachableStep {
        /// Unreachable step index.
        step: usize,
    },
    /// Expression type did not match the field contract.
    #[error("type mismatch in {field}: expected {expected}, found {found}")]
    TypeMismatch {
        /// Field being validated.
        field: &'static str,
        /// Required type.
        expected: &'static str,
        /// Inferred type.
        found: &'static str,
    },
    /// Expression referenced a slot whose type is not known at validation time.
    #[error("unknown slot type in {field}: {slot}")]
    UnknownSlotType {
        /// Field being validated.
        field: &'static str,
        /// Missing slot index.
        slot: usize,
    },
    /// Secret-tainted data cannot cross a public result boundary.
    #[error("secret-tainted value cannot be used in {field}")]
    SecretTaintLeak {
        /// Field being validated.
        field: &'static str,
    },
    /// Expression lexer found a character outside the v1 expression grammar.
    #[error("expression lex failed at byte {index} in {expression}: unexpected {found:?}")]
    ExpressionUnexpectedChar {
        /// Full source expression.
        expression: Box<str>,
        /// Byte index in the expression string.
        index: usize,
        /// Character that could not be tokenized.
        found: char,
    },
    /// Expression lexer reached EOF inside a string literal.
    #[error("expression string is unterminated at byte {index} in {expression}")]
    ExpressionUnterminatedString {
        /// Full source expression.
        expression: Box<str>,
        /// Opening quote byte index.
        index: usize,
    },
    /// Expression integer literal exceeded i64.
    #[error("expression integer is outside i64 range at byte {index} in {expression}")]
    ExpressionIntegerOutOfRange {
        /// Full source expression.
        expression: Box<str>,
        /// Literal start byte index.
        index: usize,
    },
    /// Expression exceeded a compiler-side hard bound.
    #[error("expression exceeds {limit} limit {max} in {expression}")]
    ExpressionLimitExceeded {
        /// Full source expression.
        expression: Box<str>,
        /// Limit category.
        limit: &'static str,
        /// Maximum allowed value.
        max: usize,
    },
    /// Expression parser found the wrong token shape.
    #[error("expression parse failed at byte {index} in {expression}: expected {expected}")]
    ExpressionUnexpectedToken {
        /// Full source expression.
        expression: Box<str>,
        /// Byte index in the expression string.
        index: usize,
        /// Expected syntactic element.
        expected: &'static str,
    },
    /// Expression parser does not accept bare identifiers beyond literals.
    #[error("unknown expression identifier at byte {index} in {expression}: {identifier}")]
    ExpressionUnknownIdentifier {
        /// Full source expression.
        expression: Box<str>,
        /// Byte index in the expression string.
        index: usize,
        /// Unknown identifier.
        identifier: Box<str>,
    },
    /// Expression bytecode lowering needs a later compiler/runtime table.
    #[error("expression bytecode lowering does not support {feature} yet")]
    ExpressionLoweringUnsupported {
        /// Unsupported expression feature.
        feature: &'static str,
    },
    /// Helper call has the wrong number of arguments for bytecode lowering.
    #[error("expression helper {helper} expects {expected} args, found {actual}")]
    ExpressionHelperArity {
        /// Helper name.
        helper: &'static str,
        /// Required arity.
        expected: usize,
        /// Actual argument count.
        actual: usize,
    },
    /// Side-effecting action lacks safe retry semantics.
    #[error("action {action:?} has side-effect {side_effect:?} with unsafe retry: {reason}")]
    IdempotencyViolation {
        /// Action that failed the idempotency gate.
        action: ActionId,
        /// Side-effect classification of the action.
        side_effect: SideEffect,
        /// Human-readable reason for the rejection.
        reason: Box<str>,
    },
}

