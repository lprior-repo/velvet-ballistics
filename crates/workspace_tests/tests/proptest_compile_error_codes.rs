//! Property test: CompileError symbolic code registration and consistency.
//!
//! Invariant: All CompileError symbolic codes from `.code()` are registered
//!            in CODE_REGISTRY. The code() return type is SymbolicCode.
//! Invariant: symbolic_code() is consistent with code().

use vb_compile::{CompileError, SourceMark};
use vb_core::diagnostic::{CODE_REGISTRY, HasSymbolicCode, SymbolicCode};
use vb_core::ids::ActionId;

/// Build a SourceMark with unavailable flag (tree-only validation path).
fn unavailable_mark() -> SourceMark {
    SourceMark {
        index: 0,
        end_index: 0,
        line: 1,
        column: 1,
        available: false,
    }
}

/// Enumerate a representative sample of CompileError variants.
fn compile_error_sample() -> Vec<CompileError> {
    vec![
        // YAML structural errors
        CompileError::SourceTooLarge {
            actual: 1024,
            limit: 512,
        },
        CompileError::EmptySource,
        CompileError::TopLevelNotMapping,
        CompileError::DuplicateKey {
            key: Box::<str>::from("steps"),
            mark: unavailable_mark(),
        },
        CompileError::BadValue,
        CompileError::FloatForbidden,
        // Limits
        CompileError::DepthLimit {
            depth: 64,
            limit: 63,
        },
        CompileError::NodeLimit { limit: 1000 },
        CompileError::SequenceLimit {
            actual: 500,
            limit: 200,
        },
        CompileError::MappingLimit {
            actual: 500,
            limit: 200,
        },
        CompileError::ScalarLimit {
            actual: 10000,
            limit: 1000,
        },
        // Missing / unknown fields
        CompileError::MissingField { field: "steps" },
        CompileError::UnknownTopLevelField {
            field: Box::<str>::from("extra"),
        },
        CompileError::InvalidVersion {
            actual: Box::<str>::from("999"),
        },
        CompileError::InvalidTriggerCount { count: 0 },
        CompileError::UnknownTriggerKind {
            trigger: Box::<str>::from("cron"),
        },
        // Step-level errors
        CompileError::EmptySteps,
        CompileError::InvalidName {
            field: "id",
            value: Box::<str>::from("invalid!"),
        },
        CompileError::MissingStepId { step: 1 },
        CompileError::DuplicateStepId {
            id: Box::<str>::from("dup"),
        },
        CompileError::StepShape { step: 1 },
        CompileError::UnknownStepField {
            step: 1,
            field: Box::<str>::from("extra"),
        },
        CompileError::MissingStepPrimitive { step: 1 },
        CompileError::MultipleStepPrimitives { step: 1 },
        CompileError::UnsupportedStepPrimitive {
            step: 1,
            primitive: "nonsense",
        },
        CompileError::UnsupportedStepControlField {
            step: 1,
            field: Box::<str>::from("extra"),
        },
        CompileError::MissingStepField {
            step: 1,
            field: "do",
        },
        CompileError::StepFieldShape {
            step: 1,
            field: "id",
            expected: "string",
        },
        CompileError::StepIndexOutOfRange { value: 70000 },
        CompileError::SlotIndexOutOfRange { value: -1_i64 },
        CompileError::BranchTargetOutOfRange { value: -1_i64 },
        CompileError::BackwardBranchTarget { step: 2, target: 1 },
        CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "for_each",
            field: "body",
            value: 1000,
            limit: 100,
        },
        CompileError::LastStepMustFinish,
        CompileError::UnsupportedConstantValue { step: 1 },
        // References
        CompileError::UnknownReferenceRoot {
            reference: Box::<str>::from("$foo.bar"),
            root: Box::<str>::from("foo"),
        },
        CompileError::IllegalReference {
            reference: Box::<str>::from("runtime.abc"),
        },
        CompileError::UnknownReferenceName {
            kind: "input",
            reference: Box::<str>::from("input.xyz"),
            name: Box::<str>::from("xyz"),
        },
        CompileError::UnsupportedAccessorReference {
            reference: Box::<str>::from("ctx.abc"),
            root: Box::<str>::from("ctx"),
            path: Box::<str>::from("abc"),
        },
        CompileError::UnknownStepTarget {
            step: 1,
            target: 99,
        },
        CompileError::UnknownStepLabel {
            step: 1,
            label: Box::<str>::from("missing"),
        },
        CompileError::UnreachableStep { step: 5 },
        // Type mismatch
        CompileError::TypeMismatch {
            field: "id",
            expected: "string",
            found: "integer",
        },
        CompileError::UnknownSlotType {
            field: "output",
            slot: 99,
        },
        CompileError::SecretTaintLeak { field: "output" },
        // Expression errors
        CompileError::ExpressionUnexpectedChar {
            expression: Box::<str>::from("1+@"),
            index: 2,
            found: '@',
        },
        CompileError::ExpressionUnterminatedString {
            expression: Box::<str>::from("\"hello"),
            index: 6,
        },
        CompileError::ExpressionIntegerOutOfRange {
            expression: Box::<str>::from("99999999999999999999"),
            index: 0,
        },
        CompileError::ExpressionFloatOutOfRange {
            expression: Box::<str>::from("1e999"),
            index: 0,
        },
        CompileError::ExpressionLimitExceeded {
            expression: Box::<str>::from("a+b+c+d+e+f+g+h"),
            limit: "token",
            max: 5,
        },
        CompileError::ExpressionUnexpectedToken {
            expression: Box::<str>::from("1+"),
            index: 2,
            expected: "operand",
        },
        CompileError::ExpressionUnknownIdentifier {
            expression: Box::<str>::from("x + 1"),
            index: 0,
            identifier: Box::<str>::from("x"),
        },
        CompileError::ExpressionLoweringUnsupported {
            feature: Box::<str>::from("ternary"),
        },
        CompileError::ExpressionHelperArity {
            helper: "round",
            expected: 1,
            actual: 2,
        },
        CompileError::IdempotencyViolation {
            action: ActionId::new(0),
            side_effect: vb_core::SideEffect::LocalWrite,
            reason: Box::<str>::from("unsafe retry"),
        },
        // Input schema
        CompileError::UnknownInputSchemaField {
            field: Box::<str>::from("extra"),
        },
        CompileError::InvalidInputSchema {
            field: "type",
            expected: "string",
        },
        CompileError::UnsupportedTopLevelResult,
        CompileError::UnsupportedTopLevelDeclaration { field: "secrets" },
        CompileError::DuplicateOutputName {
            name: Box::<str>::from("result"),
        },
        CompileError::UnknownOutputName {
            name: Box::<str>::from("missing"),
        },
        // Trigger field errors
        CompileError::TriggerShape {
            trigger: Box::<str>::from("http"),
            expected: "object",
        },
        CompileError::UnknownTriggerField {
            trigger: "http",
            field: Box::<str>::from("extra"),
        },
        CompileError::MissingTriggerField {
            trigger: "http",
            field: "path",
        },
        CompileError::InvalidTriggerField {
            trigger: "http",
            field: "method",
            expected: "string",
        },
        // Canonical YAML
        CompileError::CanonicalYaml {
            category: "parse_error",
            message: Box::<str>::from("syntax error"),
        },
    ]
}

#[test]
fn compile_error_code_returns_symbolic_for_all_variants_sample() {
    let errors = compile_error_sample();
    assert!(
        errors.len() >= 60,
        "must have at least 60 CompileError variant samples"
    );

    for error in &errors {
        let code = error.code();
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "CompileError code '{}' must be registered in CODE_REGISTRY",
            code.as_str()
        );
        assert!(
            CODE_REGISTRY.iter().any(|e| e.symbolic == code.as_str()),
            "CompileError code '{}' must have an entry in CODE_REGISTRY",
            code.as_str()
        );
    }
}

#[test]
fn compile_error_code_and_symbolic_code_consistent() {
    let errors = compile_error_sample();
    for error in &errors {
        let code1 = error.code();
        let code2 = HasSymbolicCode::symbolic_code(error);
        assert_eq!(
            code1, code2,
            "CompileError::code() and HasSymbolicCode::symbolic_code() must agree"
        );
    }
}

#[test]
fn compile_error_code_return_type_is_symbolic_code() {
    // Compile-time type check: code() returns SymbolicCode, not &'static str.
    fn _assert_returns_symbolic_code(_f: fn(&CompileError) -> SymbolicCode) {}
    _assert_returns_symbolic_code(CompileError::code);
}

#[test]
fn compile_error_diagnostic_code_aliases_code() {
    let error = CompileError::EmptySource;
    assert_eq!(error.code(), error.diagnostic_code());
}

#[test]
fn compile_error_compilation_specific_codes_present() {
    // Verify that all symbolic codes returned by CompileError::code()
    // for compilation-specific errors are in the registry.
    let errors = compile_error_sample();
    let mut all_codes: Vec<&str> = errors.iter().map(|e| e.code().as_str()).collect();
    all_codes.sort_unstable();
    all_codes.dedup();

    for code_name in &all_codes {
        assert!(
            SymbolicCode::from_static(code_name).is_some(),
            "CompileError symbolic code '{}' must be constructible via from_static",
            code_name
        );
    }
}
