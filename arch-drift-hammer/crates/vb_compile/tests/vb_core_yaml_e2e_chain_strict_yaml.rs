#![forbid(unsafe_code)]

use vb_compile::{CompileError, CompileErrors, YamlCompiler};

#[derive(Debug, Clone, PartialEq, Eq)]
enum StrictYamlRejection {
    DuplicateKey { key: String },
    AnchorForbidden,
    TagForbidden,
    DocumentCount { count: usize },
    TopLevelNotMapping,
}

fn first_error(source: &str) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source.as_bytes()) {
        Ok(ast) => Err(format!("strict YAML unexpectedly compiled: {ast:?}")),
        Err(CompileErrors(errors)) => match errors.into_iter().next() {
            Some(error) => Ok(error),
            None => Err("strict YAML rejection had no diagnostic".to_owned()),
        },
    }
}

fn compile_yaml(source: &str) -> Result<vb_core::CompiledWorkflow, CompileErrors> {
    vb_compile::compile_workflow(source.as_bytes())
}

fn classify(error: CompileError) -> StrictYamlRejection {
    match error {
        CompileError::DuplicateKey { key, .. } => StrictYamlRejection::DuplicateKey {
            key: key.to_string(),
        },
        CompileError::AnchorForbidden { .. } => StrictYamlRejection::AnchorForbidden,
        CompileError::TagForbidden { .. } => StrictYamlRejection::TagForbidden,
        CompileError::DocumentCount { count } => StrictYamlRejection::DocumentCount { count },
        CompileError::TopLevelNotMapping => StrictYamlRejection::TopLevelNotMapping,
        other => StrictYamlRejection::DuplicateKey {
            key: format!("unexpected:{other}"),
        },
    }
}

#[test]
fn strict_yaml_rejected_when_duplicate_top_level_key_present() -> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: first\nname: second\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(
        rejection,
        StrictYamlRejection::DuplicateKey {
            key: "name".to_owned()
        }
    );
    Ok(())
}

#[test]
fn validate_and_compile_yaml_returns_artifact_when_minimal_yaml_is_valid() -> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: valid_minimal\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";

    let workflow = compile_yaml(source).map_err(|errors| errors.to_string())?;

    assert_eq!(workflow.digest(), workflow.to_parts().digest);
    assert_eq!(workflow.name(), "valid_minimal");
    assert_eq!(workflow.node_count(), 2);
    Ok(())
}

#[test]
fn validate_and_compile_yaml_rejects_duplicate_keys_with_strict_yaml_rejected() -> Result<(), String>
{
    let source = "version: velvet-ballistics/v1\nname: first\nname: second\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(
        rejection,
        StrictYamlRejection::DuplicateKey {
            key: "name".to_owned()
        }
    );
    Ok(())
}

#[test]
fn strict_yaml_rejected_when_anchor_present() -> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: anchored\nwhen: &trigger { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::AnchorForbidden);
    Ok(())
}

#[test]
fn validate_and_compile_yaml_rejects_aliases_and_anchors_with_strict_yaml_rejected()
-> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: anchored\nwhen: &trigger { manual: {} }\nsteps:\n  - id: done\n    finish: { result: *trigger }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::AnchorForbidden);
    Ok(())
}

#[test]
fn strict_yaml_rejected_when_explicit_tag_present() -> Result<(), String> {
    let source = "version: !vb velvet-ballistics/v1\nname: tagged\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::TagForbidden);
    Ok(())
}

#[test]
fn validate_and_compile_yaml_rejects_explicit_tags_with_strict_yaml_rejected() -> Result<(), String>
{
    let source = "version: !vb velvet-ballistics/v1\nname: tagged\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::TagForbidden);
    Ok(())
}

#[test]
fn strict_yaml_rejected_when_multi_document_stream_present() -> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: first\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n---\nversion: velvet-ballistics/v1\nname: second\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::DocumentCount { count: 2 });
    Ok(())
}

#[test]
fn validate_and_compile_yaml_rejects_multi_document_stream_with_strict_yaml_rejected()
-> Result<(), String> {
    let source = "version: velvet-ballistics/v1\nname: first\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n---\nversion: velvet-ballistics/v1\nname: second\nwhen: { manual: {} }\nsteps:\n  - id: done\n    finish: { result: 0 }\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::DocumentCount { count: 2 });
    Ok(())
}

#[test]
fn strict_yaml_rejected_when_top_level_shape_is_sequence() -> Result<(), String> {
    let source = "- version\n- velvet-ballistics/v1\n";

    let rejection = classify(first_error(source)?);

    assert_eq!(rejection, StrictYamlRejection::TopLevelNotMapping);
    Ok(())
}
