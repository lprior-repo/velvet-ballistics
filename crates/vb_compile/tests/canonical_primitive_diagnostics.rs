use vb_compile::CompileError;

fn check(error: CompileError, expected: &str) {
    assert_eq!(
        (error.code().as_str(), error.diagnostic_code().as_str()),
        (expected, expected),
    );
}

fn primitive(name: &'static str) -> CompileError {
    CompileError::UnsupportedStepPrimitive {
        step: 0,
        primitive: name,
    }
}

fn shape(field: &'static str) -> CompileError {
    CompileError::StepFieldShape {
        step: 0,
        field,
        expected: "mapping",
    }
}

#[test]
fn canonical_together_primitive_has_specific_code() {
    check(primitive("together"), "INVALID_TOGETHER");
}

#[test]
fn canonical_reduce_primitive_has_specific_code() {
    check(primitive("reduce"), "INVALID_REDUCE");
}

#[test]
fn canonical_together_shape_has_specific_code() {
    check(shape("together"), "INVALID_TOGETHER");
}

#[test]
fn canonical_reduce_shape_has_specific_code() {
    check(shape("reduce"), "INVALID_REDUCE");
}

#[test]
fn existing_diagnostic_labels_and_fallbacks_keep_their_codes() {
    for (name, expected) in [
        ("parallel", "INVALID_TOGETHER"),
        ("aggregate", "INVALID_REDUCE"),
        ("summarize", "INVALID_REDUCE"),
        ("collect", "INVALID_COLLECT"),
        ("gather", "INVALID_COLLECT"),
        ("unknown", "UNKNOWN_STEP_FIELD"),
    ] {
        check(primitive(name), expected);
    }
    for (name, expected) in [
        ("parallel", "INVALID_TOGETHER"),
        ("branches", "INVALID_TOGETHER"),
        ("aggregate", "INVALID_REDUCE"),
        ("unknown", "TYPE_MISMATCH"),
    ] {
        check(shape(name), expected);
    }
}
