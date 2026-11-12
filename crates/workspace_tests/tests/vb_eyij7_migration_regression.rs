//! vb-eyij7: Migration regression tests for canonical workspace structure.
//!
//! Verifies:
//! 1. Exact workspace member equality (12 members) via toml parsing
//! 2. YAML/expression API identity and behavior parity
//! 3. ExprError conversion from CoreError preserves specific variants
//! 4. No duplicate old implementation modules
//! 5. Canonical product naming (parsed from manifests)
//! 6. Fuzz targets reachable
//! 7. cargo metadata resolves exactly 12 packages with correct names

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::path::Path;

#[path = "support/vb_eyij7_workspace_members.rs"]
mod workspace_membership;
#[path = "support/vb_eyij7_workspace_metadata.rs"]
mod workspace_metadata;

// ──────────────────────────────────────────────────────────────────────────────
// Section 1: Workspace Member Equality (exact, via toml parsing)
// ──────────────────────────────────────────────────────────────────────────────

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")?;
    let root = Path::new(&manifest)
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no workspace root"))?
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no workspace root"))?;
    Ok(root.to_path_buf())
}

type PathBuf = std::path::PathBuf;

/// Parse root Cargo.toml and extract exact [workspace].members array.
fn workspace_members() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    workspace_metadata::parse_members(&std::fs::read_to_string(root.join("Cargo.toml"))?)
}

#[test]
fn workspace_has_exactly_12_members() -> Result<(), Box<dyn std::error::Error>> {
    workspace_membership::validate_members(&workspace_members()?)?;
    Ok(())
}

#[test]
fn workspace_members_are_in_canonical_order() -> Result<(), Box<dyn std::error::Error>> {
    workspace_membership::validate_order(&workspace_members()?)?;
    Ok(())
}

#[test]
fn workspace_no_old_member_names() -> Result<(), Box<dyn std::error::Error>> {
    let actual = workspace_members()?;

    let old_names = [
        "crates/vb_cli",
        "crates/vb_queue_semantics",
        "crates/vb_boundary_inventory",
    ];
    for old in &old_names {
        if actual.iter().any(|m| m == old) {
            return Err(format!("Old workspace member name still present: {}", old).into());
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 2: cargo metadata resolves exactly 12 packages with correct names
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn cargo_metadata_resolves_exactly_12_packages() -> Result<(), Box<dyn std::error::Error>> {
    let packages = workspace_metadata::read_packages(&workspace_root()?)?;
    workspace_membership::validate_packages(&packages)?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 3: YAML API Identity and Behavior Parity
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_yaml_parse_yaml_events_is_ok_for_valid_yaml() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: test\n";
    vb_yaml::parse_yaml_events(yaml)?;
    Ok(())
}

#[test]
fn vb_yaml_rejects_duplicate_keys_with_specific_error() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: a\nversion: b\n";
    match vb_yaml::validate_yaml_profile(yaml) {
        Err(vb_yaml::YamlError::DuplicateKeyAt { key, .. }) => {
            if key.as_ref() != "version" {
                return Err(format!("Expected DuplicateKey for 'version', got key={}", key).into());
            }
            Ok(())
        }
        Err(other) => Err(format!("Expected DuplicateKey error, got {:?}", other).into()),
        Ok(_) => Err("Expected duplicate key error, got Ok".into()),
    }
}

#[test]
fn vb_yaml_rejects_anchors_with_specific_error() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "x: &anchor value\ny: *anchor\n";
    match vb_yaml::validate_yaml_profile(yaml) {
        Err(vb_yaml::YamlError::AnchorAliasMerge) => Ok(()),
        Err(other) => Err(format!("Expected AnchorAliasMerge error, got {:?}", other).into()),
        Ok(_) => Err("Expected anchor/alias error, got Ok".into()),
    }
}

#[test]
fn vb_yaml_rejects_merge_keys_with_specific_error() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "defaults: &d\n  a: 1\nitem:\n  <<: *d\n";
    match vb_yaml::validate_yaml_profile(yaml) {
        Err(vb_yaml::YamlError::AnchorAliasMerge) => Ok(()),
        Err(other) => Err(format!(
            "Expected AnchorAliasMerge error for merge key, got {:?}",
            other
        )
        .into()),
        Ok(_) => Err("Expected merge key error, got Ok".into()),
    }
}

#[test]
fn vb_yaml_rejects_multiple_documents_with_specific_error() -> Result<(), Box<dyn std::error::Error>>
{
    let yaml = "a: 1\n---\nb: 2\n";
    match vb_yaml::validate_yaml_profile(yaml) {
        Err(vb_yaml::YamlError::MultipleDocuments { count }) => {
            if count != 2 {
                return Err(format!("Expected count=2, got count={}", count).into());
            }
            Ok(())
        }
        Err(other) => Err(format!("Expected MultipleDocuments error, got {:?}", other).into()),
        Ok(_) => Err("Expected multiple document error, got Ok".into()),
    }
}

#[test]
fn vb_yaml_source_map_builds_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: test\n";
    vb_yaml::build_source_map(yaml)?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 4: Expression API Identity
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_expr_lex_returns_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = vb_expr::lexer::lex_expr("1 + 2")?;
    if tokens.len() < 3 {
        return Err(format!("Expected at least 3 tokens, got {}", tokens.len()).into());
    }
    Ok(())
}

#[test]
fn vb_expr_parse_returns_ast() -> Result<(), Box<dyn std::error::Error>> {
    let tokens = vb_expr::lexer::lex_expr("1 + 2")?;
    let _ast = vb_expr::parser::parse_expr(&tokens)?;
    Ok(())
}

#[test]
fn vb_expr_compile_returns_bytecode() -> Result<(), Box<dyn std::error::Error>> {
    let resolver = |_reference: &str| None;
    let (program, _constants) = vb_expr::compile_expr("1 + 2", &resolver)?;
    if program.ops.is_empty() {
        return Err("Expected non-empty bytecode".into());
    }
    Ok(())
}

#[test]
fn vb_expr_eval_returns_result() -> Result<(), Box<dyn std::error::Error>> {
    let resolver = |_reference: &str| None;
    let (program, constants) = vb_expr::compile_expr("19 + 23", &resolver)?;
    let value = vb_expr::eval_expr_program(&program, &[], &constants)?;
    if !matches!(value, vb_core::SlotValue::I64(42)) {
        return Err(format!("Expected 42, got {:?}", value).into());
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 5: ExprError Conversion (CRITICAL REGRESSION TEST)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn expr_error_from_core_error_preserves_division_by_zero() -> Result<(), Box<dyn std::error::Error>>
{
    let core_err = vb_core::CoreError::DivisionByZero;
    let expr_err: vb_expr::ExprError = core_err.into();
    if !matches!(expr_err, vb_expr::ExprError::DivisionByZero) {
        return Err(format!("Expected ExprError::DivisionByZero, got: {:?}", expr_err).into());
    }
    Ok(())
}

#[test]
fn expr_error_from_core_error_preserves_stack_underflow() -> Result<(), Box<dyn std::error::Error>>
{
    let core_err = vb_core::CoreError::ExpressionStackUnderflow;
    let expr_err: vb_expr::ExprError = core_err.into();
    if !matches!(expr_err, vb_expr::ExprError::StackUnderflow) {
        return Err(format!("Expected ExprError::StackUnderflow, got: {:?}", expr_err).into());
    }
    Ok(())
}

#[test]
fn expr_error_from_core_error_preserves_stack_overflow() -> Result<(), Box<dyn std::error::Error>> {
    let core_err = vb_core::CoreError::ExpressionStackOverflow { max: 64 };
    let expr_err: vb_expr::ExprError = core_err.into();
    match expr_err {
        vb_expr::ExprError::StackOverflow { max } => {
            if max != 64 {
                return Err(format!("Expected max 64, got {}", max).into());
            }
        }
        other => return Err(format!("Expected ExprError::StackOverflow, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn expr_error_from_core_error_preserves_type_mismatch() -> Result<(), Box<dyn std::error::Error>> {
    let core_err = vb_core::CoreError::TypeMismatch {
        expected: "I64",
        found: "F64",
    };
    let expr_err: vb_expr::ExprError = core_err.into();
    match expr_err {
        vb_expr::ExprError::TypeMismatch { expected, found } => {
            if expected != "I64" || found != "F64" {
                return Err(format!("Expected I64/F64, got {}/{}", expected, found).into());
            }
        }
        other => return Err(format!("Expected ExprError::TypeMismatch, got: {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn expr_error_from_core_error_preserves_non_finite() -> Result<(), Box<dyn std::error::Error>> {
    let core_err = vb_core::CoreError::NonFiniteNumber;
    let expr_err: vb_expr::ExprError = core_err.into();
    if !matches!(expr_err, vb_expr::ExprError::NonFiniteFloat) {
        return Err(format!("Expected ExprError::NonFiniteFloat, got: {:?}", expr_err).into());
    }
    Ok(())
}

#[test]
fn expr_error_from_core_error_fallback_is_unexpected_eof() -> Result<(), Box<dyn std::error::Error>>
{
    let core_err = vb_core::CoreError::InvalidProgramCounter {
        step: vb_core::StepIdx::new(0),
    };
    let expr_err: vb_expr::ExprError = core_err.into();
    if !matches!(expr_err, vb_expr::ExprError::UnexpectedEof) {
        return Err(format!("Expected ExprError::UnexpectedEof, got: {:?}", expr_err).into());
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 6: vb_compile Re-exports Work (Backward Compatibility)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_compile_reexports_expr_error() -> Result<(), Box<dyn std::error::Error>> {
    let result: Result<vb_compile::ExprError, vb_compile::ExprError> =
        Ok(vb_compile::ExprError::DivisionByZero);
    result?;
    Ok(())
}

#[test]
fn vb_compile_reexports_parse_yaml_events() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: test\n";
    vb_compile::parse_yaml_events(yaml)?;
    Ok(())
}

#[test]
fn vb_compile_reexports_parse_workflow_source() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n";
    vb_compile::parse_workflow_source(yaml)?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 7: Canonical Product Naming (parsed from manifests)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn velvet_ballistics_package_name_is_correct() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let toml_text = std::fs::read_to_string(root.join("crates/velvet_ballistics/Cargo.toml"))?;
    let parsed: toml::Value = toml::from_str(&toml_text)?;

    let pkg_name = parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| "package.name not found")?;

    if pkg_name != "velvet-ballistics" {
        return Err(format!(
            "Expected package.name = 'velvet-ballistics', got '{}'",
            pkg_name
        )
        .into());
    }

    Ok(())
}

#[test]
fn velvet_ballistics_lib_name_is_correct() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let toml_text = std::fs::read_to_string(root.join("crates/velvet_ballistics/Cargo.toml"))?;
    let parsed: toml::Value = toml::from_str(&toml_text)?;

    let lib_name = parsed
        .get("lib")
        .and_then(|l| l.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| "lib.name not found")?;

    if lib_name != "velvet_ballistics" {
        return Err(format!(
            "Expected lib.name = 'velvet_ballistics', got '{}'",
            lib_name
        )
        .into());
    }

    Ok(())
}

#[test]
fn velvet_ballistics_binary_name_is_correct() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let toml_text = std::fs::read_to_string(root.join("crates/velvet_ballistics/Cargo.toml"))?;
    let parsed: toml::Value = toml::from_str(&toml_text)?;

    let bins = parsed
        .get("bin")
        .and_then(|b| b.as_array())
        .ok_or_else(|| "[[bin]] section not found or not an array")?;

    if bins.is_empty() {
        return Err("No [[bin]] entries found".into());
    }

    let first_bin = &bins[0];
    let bin_name = first_bin
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "bin.name not found")?;

    if bin_name != "velvet-ballistics" {
        return Err(format!("Expected bin name 'velvet-ballistics', got '{}'", bin_name).into());
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 8: No Duplicate Old Implementation Modules
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn vb_compile_has_no_duplicate_expr_modules() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let compile_src = root.join("crates").join("vb_compile").join("src");
    let entries = std::fs::read_dir(&compile_src)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if name_str.starts_with("expr_")
            && name_str != "expression.rs"
            && name_str != "expression_bytecode.rs"
        {
            return Err(format!("Duplicate old expr module in vb_compile: {}", name_str).into());
        }
    }
    Ok(())
}

#[test]
fn vb_compile_has_no_duplicate_yaml_modules() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let compile_src = root.join("crates").join("vb_compile").join("src");
    let entries = std::fs::read_dir(&compile_src)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        if name_str.starts_with("yaml_") && name_str != "strict_yaml.rs" {
            return Err(format!("Duplicate old yaml module in vb_compile: {}", name_str).into());
        }
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 9: Fuzz Targets Reachable
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn fuzz_package_exists_and_is_member() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let fuzz_toml = root.join("fuzz").join("Cargo.toml");
    let toml = std::fs::read_to_string(&fuzz_toml)?;
    if toml.contains("[workspace]") {
        return Err("fuzz/Cargo.toml should not have [workspace] section".into());
    }
    Ok(())
}

#[test]
fn fuzz_targets_exist() -> Result<(), Box<dyn std::error::Error>> {
    let root = workspace_root()?;
    let fuzz_src = root.join("fuzz").join("fuzz_targets");
    let entries = std::fs::read_dir(&fuzz_src)?;
    let target_names: BTreeSet<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    let required = [
        "yaml_events",
        "expression",
        "ipc_frame",
        "journal_event",
        "compiled_ir",
    ];
    for req in &required {
        let expected = format!("{}.rs", req);
        if !target_names.contains(&expected) {
            return Err(format!("Missing required fuzz target: {}", expected).into());
        }
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 10: Expression Error Behavior Parity
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn expr_division_by_zero_error_variant() -> Result<(), Box<dyn std::error::Error>> {
    match vb_expr::eval_binary_op(
        vb_expr::lexer::BinaryOp::Div,
        vb_core::SlotValue::I64(10),
        vb_core::SlotValue::I64(0),
    ) {
        Err(vb_expr::ExprError::DivisionByZero) => Ok(()),
        other => Err(format!("Expected DivisionByZero, got {:?}", other).into()),
    }
}

#[test]
fn expr_overflow_error_variant() -> Result<(), Box<dyn std::error::Error>> {
    match vb_expr::eval_binary_op(
        vb_expr::lexer::BinaryOp::Add,
        vb_core::SlotValue::I64(i64::MAX),
        vb_core::SlotValue::I64(1),
    ) {
        Err(vb_expr::ExprError::IntegerOverflow) => Ok(()),
        other => Err(format!("Expected IntegerOverflow, got {:?}", other).into()),
    }
}

#[test]
fn expr_stack_underflow_via_bytecode() -> Result<(), Box<dyn std::error::Error>> {
    let program = vb_core::ExprProgram {
        ops: Box::new([vb_core::ExprOp::LoadSlot(vb_core::SlotIdx::new(999))]),
        max_stack: 1,
    };
    match vb_expr::eval_expr_program(&program, &[], &[]) {
        Err(vb_expr::ExprError::StackUnderflow) => Ok(()),
        other => Err(format!("Expected StackUnderflow, got {:?}", other).into()),
    }
}
