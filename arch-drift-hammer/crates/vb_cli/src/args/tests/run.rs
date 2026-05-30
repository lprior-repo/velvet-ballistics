use std::path::PathBuf;

use super::args;
use crate::args::{Command, DurabilityMode, OutputFormat, ParseError, StepTarget, parse_args};

#[test]
fn parse_run_accepts_journaled_mode_with_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "journaled",
        "--db",
        "journal-db",
    ]));
    if let Ok(Command::Run {
        workflow,
        input_bin,
        durability,
        db,
        ..
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(input_bin, PathBuf::from("input.bin"));
        assert_eq!(durability, DurabilityMode::Journaled);
        assert_eq!(db, Some(PathBuf::from("journal-db")));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_accepts_strict_mode_with_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "strict",
        "--db",
        "journal-db",
    ]));
    if let Ok(Command::Run { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::Strict);
        assert_eq!(db, Some(PathBuf::from("journal-db")));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_none_mode_keeps_db_optional() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));
    if let Ok(Command::Run { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::None);
        assert_eq!(db, None);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_none_mode_can_still_accept_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--db",
        "journal-db",
    ]));
    if let Ok(Command::Run { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::None);
        assert_eq!(db, Some(PathBuf::from("journal-db")));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Run { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_without_step_flags_produces_none_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));
    if let Ok(Command::Run { step, .. }) = parsed {
        assert!(step.is_none());
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_step_requires_step_input() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--step",
        "0",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--step-input"))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_run_with_step_flags() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--step",
        "3",
        "--step-input",
        "step-data.bin",
    ]));
    if let Ok(Command::Run {
        step: Some(target), ..
    }) = parsed
    {
        assert_eq!(target.step_id, 3);
        assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_rejects_unknown_durability_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "ephemeral",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::UnknownDurability(ref m)) if m == "ephemeral"),
        "expected UnknownDurability(ephemeral), got {parsed:?}"
    );
}

#[test]
fn parse_run_rejects_missing_input_bin() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--durability",
        "none",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--input-bin"))
    ));
}

#[test]
fn parse_run_rejects_missing_durability() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--durability"))
    ));
}

#[test]
fn step_target_holds_step_id_and_path() {
    let target = StepTarget {
        step_id: 5,
        step_input: PathBuf::from("data.bin"),
    };
    assert_eq!(target.step_id, 5);
    assert_eq!(target.step_input, PathBuf::from("data.bin"));
}

#[test]
fn parse_run_compiled_requires_workflow_and_input_bin_and_durability() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run-compiled",
        "workflow.vbir",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));
    if let Ok(Command::RunCompiled {
        workflow,
        input_bin,
        durability,
        db,
        output,
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.vbir"));
        assert_eq!(input_bin, PathBuf::from("input.bin"));
        assert_eq!(durability, DurabilityMode::None);
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_run_compiled_requires_db_for_strict_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run-compiled",
        "workflow.vbir",
        "--input-bin",
        "input.bin",
        "--durability",
        "strict",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_run_compiled_accepts_journaled_mode_with_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run-compiled",
        "workflow.vbir",
        "--input-bin",
        "input.bin",
        "--durability",
        "journaled",
        "--db",
        "journal-db",
    ]));
    if let Ok(Command::RunCompiled { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::Journaled);
        assert_eq!(db, Some(PathBuf::from("journal-db")));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_submit_requires_workflow_input_bin_db_durability() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "submit",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--db",
        "test-db",
        "--durability",
        "journaled",
    ]));
    if let Ok(Command::Submit {
        workflow,
        input_bin,
        db,
        durability,
        output,
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(input_bin, PathBuf::from("input.bin"));
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(durability, DurabilityMode::Journaled);
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_submit_rejects_unknown_durability() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "submit",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--db",
        "test-db",
        "--durability",
        "volatile",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::UnknownDurability(ref m)) if m == "volatile"),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_ipc_serve_requires_socket_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "ipc-serve",
        "--socket",
        "/tmp/vb.sock",
        "--db",
        "test-db",
    ]));
    if let Ok(Command::IpcServe { socket, db }) = parsed {
        assert_eq!(socket, PathBuf::from("/tmp/vb.sock"));
        assert_eq!(db, PathBuf::from("test-db"));
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_ipc_serve_rejects_missing_socket() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "ipc-serve",
        "--db",
        "test-db",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--socket"))
    ));
}

#[test]
fn parse_run_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--bogus",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownFlag { command: "run", .. })
    ));
}
