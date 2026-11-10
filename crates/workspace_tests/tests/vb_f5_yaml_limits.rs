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
            None => assert!(result.is_ok(), "valid source failed: {result:?}"),
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
    check_both(&source, YamlLimits { max_source_bytes: size, ..YamlLimits::default() }, None);
    check_both(&source, YamlLimits { max_source_bytes: size - 1, ..YamlLimits::default() },
        Some(CompileError::SourceTooLarge { actual: size, limit: size - 1 }));
}

#[test]
fn depth_boundary_is_shared() {
    check_both(SOURCE, YamlLimits { max_depth: 4, ..YamlLimits::default() }, None);
    check_both(SOURCE, YamlLimits { max_depth: 3, ..YamlLimits::default() },
        Some(CompileError::DepthLimit { depth: 4, limit: 3 }));
}

#[test]
fn node_boundary_is_shared() {
    let count = 15;
    check_both(SOURCE, YamlLimits { max_nodes: count, ..YamlLimits::default() }, None);
    check_both(SOURCE, YamlLimits { max_nodes: count - 1, ..YamlLimits::default() },
        Some(CompileError::NodeLimit { limit: count - 1 }));
}

#[test]
fn sequence_boundary_is_shared() {
    check_both(SOURCE, YamlLimits { max_sequence_len: 2, ..YamlLimits::default() }, None);
    check_both(SOURCE, YamlLimits { max_sequence_len: 1, ..YamlLimits::default() },
        Some(CompileError::SequenceLimit { actual: 2, limit: 1 }));
}

#[test]
fn mapping_boundary_is_shared() {
    check_both(SOURCE, YamlLimits { max_mapping_entries: 4, ..YamlLimits::default() }, None);
    check_both(SOURCE, YamlLimits { max_mapping_entries: 3, ..YamlLimits::default() },
        Some(CompileError::MappingLimit { actual: 4, limit: 3 }));
}

#[test]
fn scalar_boundary_counts_utf8_bytes() {
    let name = "é".repeat(20);
    let source = SOURCE.replace("{manual: {}}", &format!("{{event: {{type: '{name}'}}}}"));
    check_both(&source, YamlLimits { max_scalar_bytes: 40, ..YamlLimits::default() }, None);
    check_both(&source, YamlLimits { max_scalar_bytes: 39, ..YamlLimits::default() },
        Some(CompileError::ScalarLimit { actual: 40, limit: 39 }));
}

#[test]
fn malformed_profile_is_rejected_before_ast_in_both_paths() {
    for source in [SOURCE.replace("name: limits_test", "name: &alias limits_test"),
        SOURCE.replace("name: limits_test", "name: limits_test\nname: duplicate")] {
        let compiler = YamlCompiler::default();
        assert!(compiler.compile(source.as_bytes()).is_err());
        assert!(compiler.parse_ast(source.as_bytes()).is_err());
    }
}
