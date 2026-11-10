use vb_compile::{CompileError, CompileErrors, YamlCompiler, YamlLimits};

const SOURCE: &str = "version: velvet-ballistics/v1\nname: limits_test\nwhen: {manual: {}}\nsteps:\n  - id: first\n    set: {output: x, value: '1'}\n  - id: done\n    finish: {result: 0}\n";

fn check_both(source: &str, limits: YamlLimits, expected: Option<CompileError>) {
    let compiler = YamlCompiler::new(limits);
    let results = [
        compiler.compile(source.as_bytes()).map(|_| ()),
        compiler.parse_ast(source.as_bytes()).map(|_| ()),
    ];
    for result in results {
        match &expected {
            None => assert_eq!(result, Ok(()), "valid source failed: {result:?}"),
            Some(error) => assert_eq!(result, Err(CompileErrors(vec![error.clone()]))),
        }
    }
}

#[test]
fn valid_yaml_passes_both_public_paths() {
    check_both(SOURCE, YamlLimits::default(), None);
}

#[test]
fn source_bytes_include_unicode_comments() {
    let source = format!("{SOURCE}# café 🦀\n");
    let size = source.len();
    check_both(
        &source,
        YamlLimits {
            max_source_bytes: size,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        &source,
        YamlLimits {
            max_source_bytes: size - 1,
            ..YamlLimits::default()
        },
        Some(CompileError::SourceTooLarge {
            actual: size,
            limit: size - 1,
        }),
    );
}

#[test]
fn depth_boundary_is_shared() {
    check_both(
        SOURCE,
        YamlLimits {
            max_depth: 4,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        SOURCE,
        YamlLimits {
            max_depth: 3,
            ..YamlLimits::default()
        },
        Some(CompileError::DepthLimit { depth: 4, limit: 3 }),
    );
}

#[test]
fn node_boundary_is_shared() {
    let count = 15;
    check_both(
        SOURCE,
        YamlLimits {
            max_nodes: count,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        SOURCE,
        YamlLimits {
            max_nodes: count - 1,
            ..YamlLimits::default()
        },
        Some(CompileError::NodeLimit { limit: count - 1 }),
    );
}

#[test]
fn sequence_boundary_is_shared() {
    check_both(
        SOURCE,
        YamlLimits {
            max_sequence_len: 2,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        SOURCE,
        YamlLimits {
            max_sequence_len: 1,
            ..YamlLimits::default()
        },
        Some(CompileError::SequenceLimit {
            actual: 2,
            limit: 1,
        }),
    );
}

#[test]
fn mapping_boundary_is_shared() {
    check_both(
        SOURCE,
        YamlLimits {
            max_mapping_entries: 4,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        SOURCE,
        YamlLimits {
            max_mapping_entries: 3,
            ..YamlLimits::default()
        },
        Some(CompileError::MappingLimit {
            actual: 4,
            limit: 3,
        }),
    );
}

#[test]
fn scalar_boundary_counts_utf8_bytes() {
    let name = "é".repeat(20);
    let source = SOURCE.replace("{manual: {}}", &format!("{{event: {{type: '{name}'}}}}"));
    check_both(
        &source,
        YamlLimits {
            max_scalar_bytes: 40,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        &source,
        YamlLimits {
            max_scalar_bytes: 39,
            ..YamlLimits::default()
        },
        Some(CompileError::ScalarLimit {
            actual: 40,
            limit: 39,
        }),
    );
}

#[test]
fn caller_can_raise_scalar_limit_above_default_in_both_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let default_limit = YamlLimits::default().max_scalar_bytes;
    let length = default_limit
        .checked_add(1)
        .ok_or("scalar length overflow")?;
    let source = SOURCE.replace(
        "{manual: {}}",
        &format!("{{event: {{type: '{}'}}}}", "x".repeat(length)),
    );
    check_both(
        &source,
        YamlLimits {
            max_scalar_bytes: length,
            ..YamlLimits::default()
        },
        None,
    );
    check_both(
        &source,
        YamlLimits::default(),
        Some(CompileError::ScalarLimit {
            actual: length,
            limit: default_limit,
        }),
    );
    Ok(())
}

#[test]
fn malformed_profile_is_rejected_before_ast_in_both_paths() {
    let compiler = YamlCompiler::default();
    let anchored = SOURCE.replace("name: limits_test", "name: &alias limits_test");
    let compiled = compiler.compile(anchored.as_bytes()).map(|_| ());
    assert!(
        matches!(compiled, Err(CompileErrors(ref errors)) if matches!(errors.as_slice(), [CompileError::CanonicalYaml { category: "forbidden_feature", message }] if message.contains("anchor"))),
        "{compiled:?}"
    );
    let parsed = compiler.parse_ast(anchored.as_bytes()).map(|_| ());
    assert!(
        matches!(parsed, Err(CompileErrors(ref errors)) if matches!(errors.as_slice(), [CompileError::AnchorForbidden { mark }] if mark.available && mark.line == 2)),
        "{parsed:?}"
    );
    let duplicate = SOURCE.replace("name: limits_test", "name: limits_test\nname: duplicate");
    let compiled = compiler.compile(duplicate.as_bytes()).map(|_| ());
    assert!(
        matches!(compiled, Err(CompileErrors(ref errors)) if matches!(errors.as_slice(), [CompileError::CanonicalYaml { category: "duplicate_key", message }] if message.contains("name"))),
        "{compiled:?}"
    );
    let parsed = compiler.parse_ast(duplicate.as_bytes()).map(|_| ());
    assert!(
        matches!(parsed, Err(CompileErrors(ref errors)) if matches!(errors.as_slice(), [CompileError::DuplicateKey { key, mark }] if key.as_ref() == "name" && mark.available && mark.line == 3)),
        "{parsed:?}"
    );
}
