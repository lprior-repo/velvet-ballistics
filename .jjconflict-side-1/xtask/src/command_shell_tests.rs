use serde_json::Value;

#[test]
fn required_command_stdout_renders_parseable_json_line_with_exact_fields() -> anyhow::Result<()> {
    let status = xtask::placeholder_status(
        xtask::CommandFamily::AiContext,
        xtask::DeferredReason::NotImplementedInThisBead,
    )?;
    let rendered = xtask::render_structured_status(&status, xtask::OutputFormat::JsonLines)?;
    let parsed: Value = serde_json::from_str(&rendered)?;
    assert_eq!(parsed["command"], Value::String("ai-context".to_string()));
    assert_eq!(parsed["status"], Value::String("deferred".to_string()));
    assert_eq!(
        parsed["next_steps"],
        Value::Array(vec![Value::String(
            "open follow-up bead for ai-context engine integration".to_string()
        )])
    );
    Ok(())
}

#[test]
fn parser_returns_exact_command_error_variants() {
    assert_eq!(
        xtask::parse_xtask_command(["xtask".into()]),
        Err(xtask::XtaskCommandError::MissingRequiredInput {
            command: "xtask".to_string(),
            input: "command".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command(["xtask".into(), "ai-context".into(), "--bead".into()]),
        Err(xtask::XtaskCommandError::MissingRequiredInput {
            command: "ai-context".to_string(),
            input: "bead".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command([
            "xtask".into(),
            "ai-context".into(),
            "--bead".into(),
            "".into()
        ]),
        Err(xtask::XtaskCommandError::InvalidInput {
            command: "ai-context".to_string(),
            input: "bead".to_string(),
            reason: "bead id must not be empty".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command(["xtask".into(), "ai-context".into(), "--format".into()]),
        Err(xtask::XtaskCommandError::MissingRequiredInput {
            command: "ai-context".to_string(),
            input: "format".to_string()
        })
    );
    assert_eq!(
        xtask::parse_xtask_command([
            "xtask".into(),
            "ai-context".into(),
            "--format".into(),
            "yaml".into()
        ]),
        Err(xtask::XtaskCommandError::InvalidInput {
            command: "ai-context".to_string(),
            input: "format".to_string(),
            reason: "unsupported output format: yaml".to_string()
        })
    );
}

#[test]
fn renderer_and_router_return_exact_command_error_variants() {
    let status = xtask::StructuredStatus {
        command: "ai-context".to_string(),
        status: "deferred".to_string(),
        message: String::new(),
        next_steps: Vec::new(),
    };
    assert_eq!(
        xtask::render_structured_status(&status, xtask::OutputFormat::JsonLines),
        Err(xtask::XtaskCommandError::OutputRenderFailed {
            command: "ai-context".to_string(),
            reason: "structured status fields must be non-empty".to_string()
        })
    );
    let env = xtask::XtaskEnvironment {
        workspace_root: std::path::PathBuf::from("."),
        bead_id: None,
        output_format: xtask::OutputFormat::JsonLines,
        unavailable_families: vec![xtask::CommandFamily::AiContext],
    };
    assert_eq!(
        xtask::route_command(
            xtask::XtaskCommand::Required(xtask::CommandFamily::AiContext),
            &env
        ),
        Err(xtask::XtaskCommandError::Unavailable {
            command: "ai-context".to_string(),
            reason: "ai-context automation is not implemented in bead vb-kkvb".to_string()
        })
    );
}
