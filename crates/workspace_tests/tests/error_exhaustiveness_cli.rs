//! Regression test: attributed multiline enum variants must be parsed correctly.
//!
//! This is the core bug fix: the old regex `(?:^|,)\s*([A-Z][A-Za-z0-9_]*)` failed
//! to match variants on separate lines with `#[error(...)]` attributes. The fix
//! processes line by line, skipping attribute lines, matching the Python reference.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn make_test_env(name: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX epoch");
    let unique = format!("{}_{}", name, now.as_nanos());
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has parent")
        .join("target")
        .join("error-exhaustiveness-tests")
        .join(&unique);
    fs::create_dir_all(&root).expect("failed to create test environment directory");
    root
}

fn create_file(root: &PathBuf, path: &str, content: &str) -> PathBuf {
    let full_path = root.join(path);
    let dir = full_path.parent().expect("file path has parent directory");
    fs::create_dir_all(dir).expect("failed to create parent directory");
    fs::write(&full_path, content).expect("failed to write test file");
    full_path
}

fn make_enum_file(root: &PathBuf, variants: &str) -> PathBuf {
    let content = format!(
        "#[derive(Debug)]\n\
         pub enum IpcError {{\n{}\n\
         }}\n\
         impl IpcError {{\n\
         }}",
        variants
    );
    create_file(root, "crates/vb_ipc/src/error.rs", &content)
}

fn make_oracle_file(root: &PathBuf, mentions: &str) -> PathBuf {
    let content = format!(
        "pub fn assert_typed_ipc_error(error: IpcError) -> bool {{\n\
         use IpcError::*;\n\
         match error {{\n{}\n\
         }}\n\
         }}",
        mentions
    );
    create_file(root, "fuzz/src/ipc_target.rs", &content)
}

fn make_wildcard_oracle(root: &PathBuf, wildcard: &str) -> PathBuf {
    let typed_content = format!(
        "pub fn assert_typed_ipc_error(error: IpcError) -> bool {{\n\
         use IpcError::*;\n\
         match error {{\n{}\n\
         }}\n\
         }}",
        wildcard
    );
    create_file(root, "fuzz/src/ipc_target.rs", &typed_content);
    // Also update the proptest oracle to avoid false failures
    let proptest_content = format!(
        "pub fn assert_known_ipc_error(error: IpcError) -> bool {{\n\
         use IpcError::*;\n\
         match error {{\n{}\n\
         }}\n\
         }}",
        wildcard
    );
    create_file(
        root,
        "fuzz/tests/proptest_ipc_error_exhaustiveness.rs",
        &proptest_content,
    );
    root.join("fuzz/src/ipc_target.rs")
}

fn make_full_oracle(root: &PathBuf, mentions: &str) {
    let typed_content = format!(
        "pub fn assert_typed_ipc_error(error: IpcError) -> bool {{\n\
         use IpcError::*;\n\
         match error {{\n{}\n\
         }}\n\
         }}",
        mentions
    );
    create_file(root, "fuzz/src/ipc_target.rs", &typed_content);
    // Proptest oracle uses different function name
    let proptest_content = format!(
        "pub fn assert_known_ipc_error(error: IpcError) -> bool {{\n\
         use IpcError::*;\n\
         match error {{\n{}\n\
         }}\n\
         }}",
        mentions
    );
    create_file(
        root,
        "fuzz/tests/proptest_ipc_error_exhaustiveness.rs",
        &proptest_content,
    );
}

fn make_stubs(root: &PathBuf) {
    // JournalError enum with Foo variant
    create_file(
        root,
        "crates/vb_storage/src/error/mod.rs",
        "#[derive(Debug)]\npub enum JournalError { Foo }\n",
    );
    // JournalError oracles (4 of them) - must mention Foo
    let journal_oracles = [
        "fuzz/src/journal_target/errors.rs",
        "fuzz/fuzz_targets/decode_record.rs",
        "fuzz/fuzz_targets/journal_decode.rs",
        "fuzz/tests/proptest_journal_error_exhaustiveness.rs",
    ];
    for (i, path) in journal_oracles.iter().enumerate() {
        let fn_name = if i == 3 {
            "assert_known_journal_error"
        } else {
            "assert_typed_journal_error"
        };
        create_file(
            root,
            path,
            &format!(
                "pub fn {}(e: JournalError) -> bool {{ use JournalError::Foo; matches!(e, Foo) }}",
                fn_name
            ),
        );
    }
    // IpcError second oracle (proptest) - will be overridden by test-specific oracle
    create_file(
        root,
        "fuzz/tests/proptest_ipc_error_exhaustiveness.rs",
        "pub fn assert_known_ipc_error(e: IpcError) -> bool { true }",
    );
    // ValidationError enum with Bar variant
    create_file(
        root,
        "crates/vb_validate/src/validation_errors.rs",
        "#[derive(Debug)]\npub enum ValidationError { Bar }\n",
    );
    // ValidationError oracle - must mention Bar
    create_file(
        root,
        "fuzz/src/validation_target/verifier.rs",
        "pub fn assert_typed_validation_error(e: ValidationError) -> bool { use ValidationError::Bar; matches!(e, Bar) }",
    );
}

#[test]
fn attributed_multiline_enum_variants_are_parsed() {
    let root = make_test_env("attributed_multiline");
    make_stubs(&root);
    // Production-like: each variant on its own line with #[error(...)] attribute
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n\
         #[error(\"too large\")]\n\
         PayloadTooLarge,\n",
    );
    // Oracle mentions all three with qualified names
    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::Disconnected => true,\n\
         IpcError::PayloadTooLarge => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");

    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn removing_required_mention_fails() {
    let root = make_test_env("removing_required_mention");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n\
         #[error(\"too large\")]\n\
         PayloadTooLarge,\n",
    );
    // Oracle only mentions two — PayloadTooLarge is missing
    make_oracle_file(
        &root,
        "Full => true,\n\
         Disconnected => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("PayloadTooLarge"),
        "Expected PayloadTooLarge in error. stderr: {}",
        stderr
    );
}

#[test]
fn empty_enum_fails() {
    let root = make_test_env("empty_enum");
    make_stubs(&root);
    make_enum_file(&root, "");
    make_oracle_file(&root, "Full => true");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("no variants parsed") || stderr.contains("parse failed"),
        "Expected empty variant error. stderr: {}",
        stderr
    );
}

#[test]
fn enum_with_only_attributes_fails() {
    let root = make_test_env("enum_only_attributes");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         // not a variant\n\
         // another comment\n",
    );
    make_oracle_file(&root, "Full => true");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("no variants parsed") || stderr.contains("parse failed"),
        "Expected empty variant error. stderr: {}",
        stderr
    );
}

#[test]
fn variants_with_data_fields_are_parsed() {
    let root = make_test_env("variants_with_data");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"size mismatch\")]\n\
         SizeMismatch { expected: usize, actual: usize },\n\
         #[error(\"version error\")]\n\
         VersionError(u16),\n",
    );
    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::SizeMismatch { .. } => true,\n\
         IpcError::VersionError(_) => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn block_and_line_comments_are_stripped() {
    let root = make_test_env("comments_stripped");
    make_stubs(&root);
    let content = r#"#[derive(Debug)]
pub enum IpcError {
    // Full queue
    #[error("full")]
    Full,
    /* Block comment
       spanning multiple lines */
    #[error("disconnected")]
    Disconnected,
}
impl IpcError {
    // method here
}"#;
    let dir = root.join("crates/vb_ipc/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("error.rs");
    fs::write(&path, content).unwrap();

    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::Disconnected => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn graceful_wildcard_empty_braces_fails() {
    let root = make_test_env("wildcard_empty_braces");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n",
    );
    // Oracle uses graceful wildcard: _ => {}
    make_wildcard_oracle(&root, "_ => {}");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("graceful wildcard"),
        "Expected wildcard error. stderr: {}",
        stderr
    );
}

#[test]
fn graceful_wildcard_ok_empty_tuple_fails() {
    let root = make_test_env("wildcard_ok_empty_tuple");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n",
    );
    make_wildcard_oracle(&root, "_ => Ok(())");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("graceful wildcard"),
        "Expected wildcard error. stderr: {}",
        stderr
    );
}

#[test]
fn graceful_wildcard_empty_tuple_fails() {
    let root = make_test_env("wildcard_empty_tuple");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n",
    );
    make_wildcard_oracle(&root, "_ => ()");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("graceful wildcard"),
        "Expected wildcard error. stderr: {}",
        stderr
    );
}

#[test]
fn enum_not_found_fails() {
    let root = make_test_env("enum_not_found");
    make_stubs(&root);
    let dir = root.join("crates/vb_ipc/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("error.rs");
    fs::write(&path, "// no enum here\n").unwrap();
    make_oracle_file(&root, "Full => true");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
}

#[test]
fn function_not_found_in_oracle_fails() {
    let root = make_test_env("function_not_found");
    make_stubs(&root);
    make_enum_file(&root, "#[error(\"full\")]\nFull,\n");
    let dir = root.join("fuzz/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("ipc_target.rs");
    fs::write(&path, "// no function here\n").unwrap();

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
}

#[test]
fn missing_enum_file_fails() {
    let root = make_test_env("missing_enum_file");
    make_stubs(&root);
    make_oracle_file(&root, "Full => true");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(!result.status.success(), "Expected failure");
}

#[test]
fn variant_count_in_success_output() {
    let root = make_test_env("variant_count");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n\
         #[error(\"too large\")]\n\
         PayloadTooLarge,\n\
         #[error(\"timeout\")]\n\
         Timeout,\n",
    );
    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::Disconnected => true,\n\
         IpcError::PayloadTooLarge => true,\n\
         IpcError::Timeout => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("4 variants"),
        "Expected variant count 4 in output. stdout: {}",
        stdout
    );
}

#[test]
fn duplicate_mentions_count_once() {
    let root = make_test_env("duplicate_mentions");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n",
    );
    // Oracle mentions Full twice — should still pass (set behavior)
    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::Full => true,\n\
         IpcError::Disconnected => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn mention_outside_oracle_function_not_counted() {
    let root = make_test_env("mention_outside_function");
    make_stubs(&root);
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n",
    );
    // Oracle has Disconnected mentioned in a comment (not in function body)
    // and only Full mentioned in the actual match arms
    let content = "pub fn assert_typed_ipc_error(error: IpcError) -> bool {\n\
         use IpcError::*;\n\
         match error {\n\
             Full => true\n\
         }\n\
     }\n\
     // Disconnected should be handled\n\
     fn other_function() {\n\
         // IpcError::Disconnected is mentioned here but not in oracle\n\
     }";
    let dir = root.join("fuzz/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("ipc_target.rs");
    fs::write(&path, content).unwrap();

    // Also update the proptest oracle
    let proptest_content = "pub fn assert_known_ipc_error(error: IpcError) -> bool {\n\
         use IpcError::*;\n\
         match error {\n\
             Full => true\n\
         }\n\
     }";
    let proptest_dir = root.join("fuzz/tests");
    let _ = fs::create_dir_all(&proptest_dir);
    let proptest_path = proptest_dir.join("proptest_ipc_error_exhaustiveness.rs");
    fs::write(&proptest_path, proptest_content).unwrap();

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );
    assert!(
        stderr.contains("Disconnected"),
        "Expected Disconnected in error. stderr: {}",
        stderr
    );
}

#[test]
fn nested_braces_in_enum_body() {
    let root = make_test_env("nested_enum_braces");
    make_stubs(&root);
    // Enum with nested braces in variant data
    let content = "#[derive(Debug)]\npub enum IpcError {\n    Full,\n    Config { params: HashMap<String, String> }\n}\nimpl IpcError {}\n";
    let dir = root.join("crates/vb_ipc/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("error.rs");
    fs::write(&path, content).unwrap();

    // Oracle mentions both variants
    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::Config { .. } => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn missing_root_argument_fails() {
    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary).output().expect("failed to run binary");

    assert!(
        !result.status.success(),
        "Expected failure without root argument"
    );
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        stderr.contains("Missing required root path argument"),
        "Expected root argument error. stderr: {}",
        stderr
    );
}

#[test]
fn duplicate_enum_variant_names_deduplicated() {
    let root = make_test_env("duplicate_enum_variants");
    make_stubs(&root);
    // Enum with duplicate variant names — should deduplicate
    let content = "#[derive(Debug)]\npub enum IpcError {\n    Full,\n    Full,\n    Disconnected,\n    Disconnected,\n    Timeout\n}\nimpl IpcError {}\n";
    let dir = root.join("crates/vb_ipc/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("error.rs");
    fs::write(&path, content).unwrap();

    // Oracle mentions all three unique variants
    make_full_oracle(
        &root,
        "IpcError::Full => true,\n\
         IpcError::Disconnected => true,\n\
         IpcError::Timeout => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success with deduplicated variants. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8_lossy(&result.stdout);
    // Should report 3 unique variants, not 5
    assert!(
        stdout.contains("3 variants"),
        "Expected variant count 3 (deduplicated). stdout: {}",
        stdout
    );
}

#[test]
fn multiline_struct_field_variants_parsed() {
    let root = make_test_env("multiline_struct_fields");
    make_stubs(&root);
    // Genuinely multiline struct variant with uppercase field types
    let content = "#[derive(Debug)]\npub enum IpcError {\n\
    #[error(\"size mismatch\")]\n\
    SizeMismatch {\n\
        expected: EventSeq,\n\
        actual: RunId,\n\
    },\n\
    #[error(\"version\")]\n\
    VersionError(u16),\n\
}\nimpl IpcError {}\n";
    let dir = root.join("crates/vb_ipc/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("error.rs");
    fs::write(&path, content).unwrap();

    make_full_oracle(
        &root,
        "IpcError::SizeMismatch { .. } => true,\n\
         IpcError::VersionError(_) => true\n",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success with multiline struct variants. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn unrelated_function_mention_not_counted() {
    let root = make_test_env("unrelated_function");
    make_stubs(&root);
    // Enum with three variants
    make_enum_file(
        &root,
        "#[error(\"full\")]\n\
         Full,\n\
         #[error(\"disconnected\")]\n\
         Disconnected,\n\
         #[error(\"timeout\")]\n\
         Timeout,\n",
    );

    // Oracle mentions Full and Timeout, but NOT Disconnected
    // Disconnected is mentioned in a SEPARATE unrelated function
    let content = "pub fn assert_typed_ipc_error(error: IpcError) -> bool {\n\
         use IpcError::*;\n\
         match error {\n\
             IpcError::Full => true,\n\
             IpcError::Timeout => true\n\
         }\n\
     }\n\
     fn other_unrelated_function() {\n\
         let e = IpcError::Disconnected;\n\
         // This mention should NOT count toward oracle coverage\n\
     }\n\
     // IpcError::Disconnected in a comment also should not count\n";
    let dir = root.join("fuzz/src");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("ipc_target.rs");
    fs::write(&path, content).unwrap();

    // Proptest oracle mentions all three (so it passes)
    let proptest_content = "pub fn assert_known_ipc_error(error: IpcError) -> bool {\n\
         use IpcError::*;\n\
         match error {\n\
             IpcError::Full => true,\n\
             IpcError::Disconnected => true,\n\
             IpcError::Timeout => true\n\
         }\n\
     }";
    let proptest_dir = root.join("fuzz/tests");
    let _ = fs::create_dir_all(&proptest_dir);
    let proptest_path = proptest_dir.join("proptest_ipc_error_exhaustiveness.rs");
    fs::write(&proptest_path, proptest_content).unwrap();

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure because Disconnected not in oracle. stderr: {}",
        stderr
    );
    // Should report Disconnected as missing in the oracle function
    assert!(
        stderr.contains("Disconnected"),
        "Expected Disconnected in error. stderr: {}",
        stderr
    );
    // Should NOT report Full as missing (it IS in the oracle)
    assert!(
        !stderr.contains("Full"),
        "Full should not be reported as missing. stderr: {}",
        stderr
    );
}

#[test]
fn all_seven_output_count_lines() {
    let root = &make_test_env("seven_count_lines");

    // JournalError with 5 variants (multiline enum, fully qualified oracle mentions)
    let journal_enum = "#[derive(Debug)]\n\
        pub enum JournalError {\n\
        #[error(\"foo\")]\n\
        Foo,\n\
        #[error(\"bar\")]\n\
        Bar,\n\
        #[error(\"baz\")]\n\
        Baz,\n\
        #[error(\"qux\")]\n\
        Qux,\n\
        #[error(\"quux\")]\n\
        Quux\n\
        }\n";
    create_file(root, "crates/vb_storage/src/error/mod.rs", journal_enum);

    let journal_oracles = [
        "fuzz/src/journal_target/errors.rs",
        "fuzz/fuzz_targets/decode_record.rs",
        "fuzz/fuzz_targets/journal_decode.rs",
        "fuzz/tests/proptest_journal_error_exhaustiveness.rs",
    ];
    for (i, path) in journal_oracles.iter().enumerate() {
        let fn_name = if i == 3 {
            "assert_known_journal_error"
        } else {
            "assert_typed_journal_error"
        };
        let oracle_content = &format!(
            "pub fn {}(e: JournalError) -> bool {{\n\
            use JournalError::*;\n\
            matches!(e, JournalError::Foo | JournalError::Bar | JournalError::Baz | JournalError::Qux | JournalError::Quux)\n\
        }}",
            fn_name
        );
        create_file(root, path, oracle_content);
    }

    // IpcError with 3 variants
    let ipc_enum = "#[derive(Debug)]\n\
        pub enum IpcError {\n\
        #[error(\"a\")]\n\
        A,\n\
        #[error(\"b\")]\n\
        B,\n\
        #[error(\"c\")]\n\
        C\n\
        }\n";
    create_file(root, "crates/vb_ipc/src/error.rs", ipc_enum);
    let ipc_oracles = [
        "fuzz/src/ipc_target.rs",
        "fuzz/tests/proptest_ipc_error_exhaustiveness.rs",
    ];
    for (i, path) in ipc_oracles.iter().enumerate() {
        let fn_name = if i == 0 {
            "assert_typed_ipc_error"
        } else {
            "assert_known_ipc_error"
        };
        let oracle_content = &format!(
            "pub fn {}(e: IpcError) -> bool {{\n\
            use IpcError::*;\n\
            matches!(e, IpcError::A | IpcError::B | IpcError::C)\n\
        }}",
            fn_name
        );
        create_file(root, path, oracle_content);
    }

    // ValidationError with 2 variants
    let val_enum = "#[derive(Debug)]\n\
        pub enum ValidationError {\n\
        #[error(\"x\")]\n\
        X,\n\
        #[error(\"y\")]\n\
        Y\n\
        }\n";
    create_file(
        root,
        "crates/vb_validate/src/validation_errors.rs",
        val_enum,
    );
    let val_oracle = "pub fn assert_typed_validation_error(e: ValidationError) -> bool {\n\
        use ValidationError::*;\n\
        matches!(e, ValidationError::X | ValidationError::Y)\n\
    }";
    create_file(root, "fuzz/src/validation_target/verifier.rs", val_oracle);

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert!(
        result.status.success(),
        "Expected success. stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);

    // JournalError: 4 oracles × 5 variants
    assert!(
        stdout.contains("JournalError in fuzz/src/journal_target/errors.rs::assert_typed_journal_error: 5 variants")
            && stdout.contains("JournalError in fuzz/fuzz_targets/decode_record.rs::assert_typed_journal_error: 5 variants")
            && stdout.contains("JournalError in fuzz/fuzz_targets/journal_decode.rs::assert_typed_journal_error: 5 variants")
            && stdout.contains("JournalError in fuzz/tests/proptest_journal_error_exhaustiveness.rs::assert_known_journal_error: 5 variants"),
        "Expected 4 JournalError lines with 5 variants. stdout: {}",
        stdout
    );

    // IpcError: 2 oracles × 3 variants
    assert!(
        stdout.contains("IpcError in fuzz/src/ipc_target.rs::assert_typed_ipc_error: 3 variants")
            && stdout.contains("IpcError in fuzz/tests/proptest_ipc_error_exhaustiveness.rs::assert_known_ipc_error: 3 variants"),
        "Expected 2 IpcError lines with 3 variants. stdout: {}",
        stdout
    );

    // ValidationError: 1 oracle × 2 variants
    assert!(
        stdout.contains("ValidationError in fuzz/src/validation_target/verifier.rs::assert_typed_validation_error: 2 variants"),
        "Expected 1 ValidationError line with 2 variants. stdout: {}",
        stdout
    );

    // Total: 7 lines
    let lines: Vec<&str> = stdout.lines().collect();
    let ok_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.contains("variants"))
        .copied()
        .collect();
    assert_eq!(
        ok_lines.len(),
        7,
        "Expected exactly 7 variant count lines. stdout: {}",
        stdout
    );
}

#[test]
fn multiple_independent_failures_all_reported() {
    let root = &make_test_env("multiple_failures");

    // JournalError: parse failure (empty enum)
    create_file(
        root,
        "crates/vb_storage/src/error/mod.rs",
        "#[derive(Debug)]\npub enum JournalError {}\n",
    );
    for (i, path) in [
        "fuzz/src/journal_target/errors.rs",
        "fuzz/fuzz_targets/decode_record.rs",
        "fuzz/fuzz_targets/journal_decode.rs",
        "fuzz/tests/proptest_journal_error_exhaustiveness.rs",
    ]
    .iter()
    .enumerate()
    {
        let fn_name = if i == 3 {
            "assert_known_journal_error"
        } else {
            "assert_typed_journal_error"
        };
        create_file(
            root,
            path,
            &format!("pub fn {}(e: JournalError) -> bool {{ true }}", fn_name),
        );
    }

    // IpcError: coverage failure (missing variant) — multiline enum, fully qualified mentions
    let ipc_enum = "#[derive(Debug)]\n\
        pub enum IpcError {\n\
        #[error(\"a\")]\n\
        A,\n\
        #[error(\"b\")]\n\
        B,\n\
        #[error(\"c\")]\n\
        C\n\
        }\n";
    create_file(root, "crates/vb_ipc/src/error.rs", ipc_enum);
    create_file(
        root,
        "fuzz/src/ipc_target.rs",
        "pub fn assert_typed_ipc_error(e: IpcError) -> bool {\n\
        use IpcError::*;\n\
        matches!(e, IpcError::A | IpcError::B)\n\
    }",
    );
    create_file(
        root,
        "fuzz/tests/proptest_ipc_error_exhaustiveness.rs",
        "pub fn assert_known_ipc_error(e: IpcError) -> bool {\n\
        use IpcError::*;\n\
        matches!(e, IpcError::A | IpcError::B | IpcError::C)\n\
    }",
    );

    // ValidationError: success
    let val_enum = "#[derive(Debug)]\n\
        pub enum ValidationError {\n\
        #[error(\"x\")]\n\
        X\n\
        }\n";
    create_file(
        root,
        "crates/vb_validate/src/validation_errors.rs",
        val_enum,
    );
    create_file(
        root,
        "fuzz/src/validation_target/verifier.rs",
        "pub fn assert_typed_validation_error(e: ValidationError) -> bool {\n\
        use ValidationError::*;\n\
        matches!(e, ValidationError::X)\n\
    }",
    );

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    assert_eq!(
        result.status.code(),
        Some(1),
        "Expected exit code 1. Got: {:?}",
        result.status.code()
    );

    let stderr = String::from_utf8_lossy(&result.stderr);

    // JournalError parse failure should be reported
    assert!(
        stderr.contains("JournalError enum parse failed"),
        "Expected JournalError enum parse failed. stderr: {}",
        stderr
    );

    // IpcError coverage failure should be reported — only C missing in first oracle
    assert!(
        stderr.contains("IpcError missing in fuzz/src/ipc_target.rs::assert_typed_ipc_error: C"),
        "Expected C missing in first IpcError oracle. stderr: {}",
        stderr
    );

    // IpcError second oracle should be complete (no missing variants reported)
    assert!(
        !stderr.contains("proptest_ipc_error_exhaustiveness"),
        "Second IpcError oracle should be complete. stderr: {}",
        stderr
    );

    // ValidationError should still have passed (its OK line should appear in stdout)
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("ValidationError in fuzz/src/validation_target/verifier.rs::assert_typed_validation_error: 1 variants"),
        "Expected ValidationError OK line. stdout: {}",
        stdout
    );
}

#[test]
fn sorted_missing_variants_in_error_message() {
    let root = make_test_env("sorted_missing");
    make_stubs(&root);
    // Enum variants in reverse alphabetical order
    make_enum_file(
        &root,
        "#[error(\"z\")]\n\
         Zebra,\n\
         #[error(\"y\")]\n\
         Yellow,\n\
         #[error(\"x\")]\n\
         Xray,\n\
         #[error(\"w\")]\n\
         Whiskey,\n",
    );

    // Oracle only mentions the first one (Zebra)
    make_oracle_file(&root, "Zebra => true");

    let binary = env!("CARGO_BIN_EXE_check-error-exhaustiveness");
    let result = Command::new(binary)
        .args([root.to_str().unwrap()])
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(
        !result.status.success(),
        "Expected failure. stderr: {}",
        stderr
    );

    // Missing variants should be reported in sorted order
    assert!(
        stderr.contains("Whiskey, Xray, Yellow"),
        "Expected sorted missing variants. stderr: {}",
        stderr
    );
}
