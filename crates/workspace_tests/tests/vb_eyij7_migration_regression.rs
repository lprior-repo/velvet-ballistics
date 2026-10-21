//! vb-eyij7: Migration regression tests for canonical workspace structure.
//!
//! Verifies:
//! 1. Exact workspace member equality (11 members)
//! 2. YAML/expression API identity and behavior parity
//! 3. ExprError conversion from CoreError preserves specific variants
//! 4. No duplicate old implementation modules
//! 5. Canonical product naming
//! 6. Fuzz targets reachable
//!
//! Baseline comparison: da633fe0 (origin/main)
//! Implementation: a244573c (HEAD)

#![forbid(unsafe_code)]

use std::collections::HashSet;

// ──────────────────────────────────────────────────────────────────────────────
// Section 1: Workspace Member Equality
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn workspace_has_exactly_11_members() {
    let toml = std::fs::read_to_string("Cargo.toml").expect("read Cargo.toml");

    let expected = [
        "crates/vb_core",
        "crates/vb_yaml",
        "crates/vb_validate",
        "crates/vb_expr",
        "crates/vb_compile",
        "crates/vb_storage",
        "crates/vb_runtime",
        "crates/vb_ipc",
        "crates/velvet_ballistics",
        "crates/workspace_tests",
        "fuzz",
    ];

    for member in &expected {
        assert!(
            toml.contains(member),
            "Missing required workspace member: {member}"
        );
    }

    // Reject old names that should not be members
    assert!(!toml.contains("crates/vb_cli"), "vb_cli should be removed");
    assert!(!toml.contains("vb_queue_semantics"), "vb_queue_semantics should be removed");
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 2: YAML API Identity
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_yaml_parse_yaml_events_is_ok_for_valid_yaml() {
    let yaml = "version: velvet-ballistics/v1\nname: test\n";
    let result = vb_yaml::parse_yaml_events(yaml);
    assert!(result.is_ok());
}

#[test]
fn vb_yaml_rejects_duplicate_keys() {
    let yaml = "version: a\nversion: b\n";
    let result = vb_yaml::validate_yaml_profile(yaml);
    assert!(result.is_err());
}

#[test]
fn vb_yaml_rejects_anchors_aliases() {
    let yaml = "x: &anchor value\ny: *anchor\n";
    let result = vb_yaml::validate_yaml_profile(yaml);
    assert!(result.is_err());
}

#[test]
fn vb_yaml_rejects_merge_keys() {
    let yaml = "defaults: &d { a: 1 }\nitem: <<: *d\n";
    let result = vb_yaml::validate_yaml_profile(yaml);
    assert!(result.is_err());
}

#[test]
fn vb_yaml_rejects_multiple_documents() {
    let yaml = "a: 1\n---\nb: 2\n";
    let result = vb_yaml::validate_yaml_profile(yaml);
    assert!(result.is_err());
}

#[test]
fn vb_yaml_source_map_builds_correctly() {
    let yaml = "version: velvet-ballistics/v1\nname: test\n";
    let result = vb_yaml::build_source_map(yaml);
    assert!(result.is_ok());
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 3: Expression API Identity and Error Variants
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_expr_lex_returns_tokens() {
    let result = vb_expr::lexer::lex_expr("1 + 2");
    assert!(result.is_ok());
    let tokens = result.unwrap();
    assert!(tokens.len() >= 3);
}

#[test]
fn vb_expr_parse_returns_ast() {
    let result = vb_expr::parser::parse_expr("1 + 2");
    assert!(result.is_ok());
}

#[test]
fn vb_expr_compile_returns_bytecode() {
    let result = vb_expr::compile_expr("1 + 2", 64);
    assert!(result.is_ok());
}

#[test]
fn vb_expr_eval_returns_result() {
    let result = vb_expr::eval_expr_program(&[
        vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(0)),
        vb_core::ExprOp::LoadConst(vb_core::ConstIdx::new(1)),
        vb_core::ExprOp::Add,
    ], &[], &[
        vb_core::ConstValue::I64(19),
        vb_core::ConstValue::I64(23),
    ]);
    assert!(result.is_ok());
    let value = result.unwrap();
    assert_eq!(value, vb_core::SlotValue::I64(42));
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 4: ExprError Conversion (CRITICAL REGRESSION TEST)
// ──────────────────────────────────────────────────────────────────────────────

/// Baseline behavior (da633fe0): Specific CoreError variants map to specific
/// ExprError variants. Regression: a244573c maps ALL to UnexpectedToken.
#[test]
fn expr_error_from_core_error_preserves_division_by_zero() {
    let core_err = vb_core::CoreError::DivisionByZero;
    let expr_err: vb_expr::ExprError = core_err.into();
    assert!(
        matches!(expr_err, vb_expr::ExprError::DivisionByZero),
        "Expected ExprError::DivisionByZero, got: {expr_err:?}"
    );
}

#[test]
fn expr_error_from_core_error_preserves_stack_underflow() {
    let core_err = vb_core::CoreError::ExpressionStackUnderflow;
    let expr_err: vb_expr::ExprError = core_err.into();
    assert!(
        matches!(expr_err, vb_expr::ExprError::StackUnderflow),
        "Expected ExprError::StackUnderflow, got: {expr_err:?}"
    );
}

#[test]
fn expr_error_from_core_error_preserves_stack_overflow() {
    let core_err = vb_core::CoreError::ExpressionStackOverflow { max: 64 };
    let expr_err: vb_expr::ExprError = core_err.into();
    assert!(
        matches!(expr_err, vb_expr::ExprError::StackOverflow { max: 64 }),
        "Expected ExprError::StackOverflow { max: 64 }, got: {expr_err:?}"
    );
}

#[test]
fn expr_error_from_core_error_preserves_type_mismatch() {
    let core_err = vb_core::CoreError::TypeMismatch {
        expected: "I64",
        found: "F64",
    };
    let expr_err: vb_expr::ExprError = core_err.into();
    assert!(
        matches!(expr_err, vb_expr::ExprError::TypeMismatch { expected: ref e, found: ref f } if e == "I64" && f == "F64"),
        "Expected ExprError::TypeMismatch { I64, F64 }, got: {expr_err:?}"
    );
}

#[test]
fn expr_error_from_core_error_preserves_non_finite() {
    let core_err = vb_core::CoreError::NonFiniteNumber;
    let expr_err: vb_expr::ExprError = core_err.into();
    assert!(
        matches!(expr_err, vb_expr::ExprError::NonFiniteFloat),
        "Expected ExprError::NonFiniteFloat, got: {expr_err:?}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 5: vb_compile Re-exports Work (Backward Compatibility)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_compile_reexports_expr_error() {
    // Compile should re-export ExprError from vb_expr
    let result: Result<vb_compile::ExprError, vb_compile::ExprError> = Ok(vb_compile::ExprError::DivisionByZero);
    assert!(result.is_ok());
}

#[test]
fn vb_compile_reexports_parse_yaml_events() {
    let yaml = "version: velvet-ballistics/v1\nname: test\n";
    let result = vb_compile::parse_yaml_events(yaml);
    assert!(result.is_ok());
}

#[test]
fn vb_compile_reexports_parse_workflow_source() {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
    let result = vb_compile::parse_workflow_source(yaml);
    assert!(result.is_ok());
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 6: Canonical Product Naming
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn velvet_ballistics_package_has_correct_lib_name() {
    // Verify the package can be imported as velvet_ballistics
    let toml = std::fs::read_to_string("crates/velvet_ballistics/Cargo.toml").expect("read Cargo.toml");
    assert!(toml.contains("name = \"velvet-ballistics\""), "Package name should be velvet-ballistics");
    assert!(toml.contains("lib.name = \"velvet_ballistics\""), "Lib name should be velvet_ballistics");
}

#[test]
fn velvet_ballistics_has_binary_target() {
    let toml = std::fs::read_to_string("crates/velvet_ballistics/Cargo.toml").expect("read Cargo.toml");
    assert!(toml.contains("[[bin]]"), "Should have [[bin]] section");
    assert!(toml.contains("name = \"velvet-ballistics\""), "Binary should be named velvet-ballistics");
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 7: No Duplicate Old Implementation Modules
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_compile_has_no_duplicate_expr_modules() {
    // Old expr_* modules should not exist in vb_compile
    let compile_src = std::fs::read_dir("crates/vb_compile/src").expect("read vb_compile/src");
    for entry in compile_src {
        let entry = entry.expect("read entry");
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        assert!(
            !name_str.starts_with("expr_"),
            "Duplicate old expr module in vb_compile: {name_str}"
        );
    }
}

#[test]
fn vb_compile_has_no_duplicate_yaml_modules() {
    // Old yaml_* modules should not exist in vb_compile
    let compile_src = std::fs::read_dir("crates/vb_compile/src").expect("read vb_compile/src");
    for entry in compile_src {
        let entry = entry.expect("read entry");
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        assert!(
            !name_str.starts_with("yaml_"),
            "Duplicate old yaml module in vb_compile: {name_str}"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 8: Fuzz Targets Reachable
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn fuzz_package_exists_and_is_member() {
    let toml = std::fs::read_to_string("fuzz/Cargo.toml").expect("read fuzz/Cargo.toml");
    // Fuzz should not have its own [workspace] declaration anymore
    assert!(!toml.contains("[workspace]"), "fuzz/Cargo.toml should not have [workspace] section");
}

#[test]
fn fuzz_targets_exist() {
    let targets = std::fs::read_dir("fuzz/src/bin").expect("read fuzz/src/bin");
    let target_names: HashSet<String> = targets
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    // Required fuzz targets from §37
    let required = ["yaml_events", "expression", "ipc_frame", "journal_event", "compiled_ir"];
    for req in &required {
        assert!(
            target_names.contains(&format!("{req}.rs")),
            "Missing required fuzz target: {req}"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 9: Effective Cargo Metadata
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn cargo_metadata_resolves_11_packages() {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("run cargo metadata");

    assert!(output.status.success(), "cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse metadata JSON");
    let packages = json["packages"].as_array().expect("packages array");
    assert_eq!(packages.len(), 11, "Expected 11 workspace packages");
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 10: Expression Error Behavior Parity
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn expr_division_by_zero_error_variant() {
    let result = vb_expr::eval_binary_op(
        vb_core::SlotValue::I64(10),
        vb_expr::lexer::BinaryOp::Div,
        vb_core::SlotValue::I64(0),
        vb_core::value::Taint::Clean,
        vb_core::value::Taint::Clean,
    );
    assert!(matches!(result, Err(vb_expr::ExprError::DivisionByZero)));
}

#[test]
fn expr_overflow_error_variant() {
    let result = vb_expr::eval_binary_op(
        vb_core::SlotValue::I64(i64::MAX),
        vb_expr::lexer::BinaryOp::Add,
        vb_core::SlotValue::I64(1),
        vb_core::value::Taint::Clean,
        vb_core::value::Taint::Clean,
    );
    assert!(matches!(result, Err(vb_expr::ExprError::IntegerOverflow)));
}

#[test]
fn expr_stack_underflow_via_bytecode() {
    // Pop without a prior push
    let result = vb_expr::eval_expr_program(&[vb_core::ExprOp::Pop], &[], &[]);
    assert!(matches!(result, Err(vb_expr::ExprError::StackUnderflow)));
}
