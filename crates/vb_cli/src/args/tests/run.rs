use std::path::PathBuf;

use super::args;
use crate::args::{Command, DurabilityMode, ParseError, StepTarget, parse_args};

#[test]
fn parse_run_accepts_db_for_journaled_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "journaled",
        "--db",
        "journal-db",
    ]));

    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
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
    }
}

#[test]
fn parse_run_compiled_requires_db_for_strict_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
fn parse_run_none_mode_keeps_db_optional() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));

    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::None);
        assert_eq!(db, None);
    }
}

#[test]
fn parse_run_without_step_flags_produces_none_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run { step, .. }) = parsed {
        assert!(step.is_none());
    }
}

#[test]
fn parse_run_step_requires_step_input() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
fn step_target_holds_step_id_and_path() {
    let target = StepTarget {
        step_id: 5,
        step_input: PathBuf::from("data.bin"),
    };
    assert_eq!(target.step_id, 5);
    assert_eq!(target.step_input, PathBuf::from("data.bin"));
}

#[test]
fn parse_run_with_step_flags() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run {
        step: Some(target), ..
    }) = parsed
    {
        assert_eq!(target.step_id, 3);
        assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
    }
}

#[test]
fn parse_run_rejects_unknown_durability_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
