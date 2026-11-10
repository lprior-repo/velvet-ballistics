use vb_compile::{CompileError, CompileErrors, YamlCompiler, YamlError, YamlLimits};

const SOURCE: &str = "version: velvet-ballistics/v1\nname: bom_limits\nwhen: {manual: {}}\nsteps:\n  - id: first\n    set: {output: x, value: '1'}\n  - id: done\n    finish: {result: 0}\n# café 🦀\n";

#[test]
fn explicit_limits_count_raw_bom_and_utf8_bytes() -> Result<(), Box<dyn std::error::Error>> {
    for prefix in ["", "\u{feff}"] {
        let source = format!("{prefix}{SOURCE}");
        let size = source.len();
        let at = YamlLimits {
            max_source_bytes: size,
            ..YamlLimits::default()
        };
        assert_eq!(
            vb_yaml::parse_workflow_source_with_limits(&source, &at).map(|_| ()),
            Ok(())
        );
        assert_eq!(
            vb_compile::parse_workflow_source_with_limits(&source, &at).map(|_| ()),
            Ok(())
        );
        assert_eq!(
            vb_yaml::yaml_profile::validate_yaml_profile_with_limits(&source, &at),
            Ok(())
        );
        let compiler = YamlCompiler::new(at);
        assert_eq!(compiler.compile(source.as_bytes()).map(|_| ()), Ok(()));
        assert_eq!(compiler.parse_ast(source.as_bytes()).map(|_| ()), Ok(()));

        let max = size.checked_sub(1).ok_or("empty fixture")?;
        let over = YamlLimits {
            max_source_bytes: max,
            ..at
        };
        let expected = YamlError::SourceTooLarge { size, max };
        assert_eq!(
            vb_yaml::parse_workflow_source_with_limits(&source, &over).map(|_| ()),
            Err(expected.clone())
        );
        assert_eq!(
            vb_compile::parse_workflow_source_with_limits(&source, &over).map(|_| ()),
            Err(expected.clone())
        );
        assert_eq!(
            vb_yaml::yaml_profile::validate_yaml_profile_with_limits(&source, &over),
            Err(expected)
        );
        let compiler = YamlCompiler::new(over);
        let expected = CompileError::SourceTooLarge {
            actual: size,
            limit: max,
        };
        assert_eq!(
            compiler.compile(source.as_bytes()).map(|_| ()),
            Err(CompileErrors(vec![expected.clone()]))
        );
        assert_eq!(
            compiler.parse_ast(source.as_bytes()).map(|_| ()),
            Err(CompileErrors(vec![expected]))
        );
    }
    Ok(())
}

#[test]
fn default_public_siblings_enforce_raw_bom_byte_cap() -> Result<(), Box<dyn std::error::Error>> {
    let max = YamlLimits::default().max_source_bytes;
    let mut source = format!("\u{feff}{SOURCE}#");
    let padding = max.checked_sub(source.len()).ok_or("fixture exceeds cap")?;
    source.extend(std::iter::repeat_n('x', padding));
    assert_eq!(source.len(), max);
    assert_eq!(vb_yaml::parse_workflow_source(&source).map(|_| ()), Ok(()));
    assert_eq!(
        vb_compile::parse_workflow_source(&source).map(|_| ()),
        Ok(())
    );
    assert_eq!(vb_yaml::load_fixture_source(&source).map(|_| ()), Ok(()));
    assert_eq!(vb_compile::load_fixture_source(&source).map(|_| ()), Ok(()));
    assert_eq!(vb_yaml::parse_yaml_events(&source).map(|_| ()), Ok(()));
    assert_eq!(vb_compile::parse_yaml_events(&source).map(|_| ()), Ok(()));
    assert_eq!(vb_yaml::validate_yaml_profile(&source), Ok(()));

    source.push('x');
    let expected = YamlError::SourceTooLarge {
        size: source.len(),
        max,
    };
    assert_eq!(
        vb_yaml::parse_workflow_source(&source).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(
        vb_compile::parse_workflow_source(&source).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(
        vb_yaml::load_fixture_source(&source).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(
        vb_compile::load_fixture_source(&source).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(
        vb_yaml::parse_yaml_events(&source).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(
        vb_compile::parse_yaml_events(&source).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(vb_yaml::validate_yaml_profile(&source), Err(expected));
    Ok(())
}
