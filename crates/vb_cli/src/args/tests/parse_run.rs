use super::*;

#[test]
fn parse_run_accepts_db_for_journaled_mode() {
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

    match parsed {
        Ok(Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            ..
        }) => {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(input_bin, PathBuf::from("input.bin"));
            assert_eq!(durability, DurabilityMode::Journaled);
            assert_eq!(db, Some(PathBuf::from("journal-db")));
        }
        other => panic!("expected Command::Run, got {other:?}"),
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

    match parsed {
        Ok(Command::Run { durability, db, .. }) => {
            assert_eq!(durability, DurabilityMode::None);
            assert_eq!(db, None);
        }
        other => panic!("expected Command::Run, got {other:?}"),
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
    match parsed {
        Ok(Command::Run { step, .. }) => {
            assert!(step.is_none());
        }
        other => panic!("expected Command::Run, got {other:?}"),
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
    match parsed {
        Ok(Command::Run {
            step: Some(target), ..
        }) => {
            assert_eq!(target.step_id, 3);
            assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
        }
        other => panic!("expected Command::Run, got {other:?}"),
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
