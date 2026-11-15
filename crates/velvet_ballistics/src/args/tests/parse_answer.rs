use super::*;

#[test]
fn parse_answer_preserves_step_and_adds_canonical_slot_selector() {
    let step = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--step",
        "3",
        "--value-file",
        "step.bin",
        "--db",
        "run.db",
    ]));
    assert!(
        matches!(step, Ok(Command::Answer { selector: crate::args::AnswerSelector::Step(step), value_file, .. })
        if step.get() == 3 && value_file == PathBuf::from("step.bin"))
    );
    let slot = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--slot",
        "2",
        "--value",
        "slot.bin",
        "--db",
        "run.db",
    ]));
    assert!(
        matches!(slot, Ok(Command::Answer { selector: crate::args::AnswerSelector::Slot(slot), value_file, .. })
        if slot.get() == 2 && value_file == PathBuf::from("slot.bin"))
    );
}

#[test]
fn parse_answer_rejects_mixed_selectors_and_value_modes() {
    let mixed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--step",
        "3",
        "--slot",
        "2",
        "--value-file",
        "a.bin",
        "--value",
        "b.bin",
        "--db",
        "run.db",
    ]));
    assert!(
        matches!(mixed, Err(ParseError::InvalidArgument(ref reason)) if reason == "--step and --slot cannot be combined")
    );
    let wrong_file = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--slot",
        "2",
        "--value-file",
        "a.bin",
        "--db",
        "run.db",
    ]));
    assert!(
        matches!(wrong_file, Err(ParseError::InvalidArgument(ref reason)) if reason == "--value-file does not match --value")
    );
}

#[test]
fn parse_answer_rejects_slot_overflow_and_duplicate_selector() {
    let overflow = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--slot",
        "65536",
        "--value",
        "slot.bin",
        "--db",
        "run.db",
    ]));
    assert!(
        matches!(overflow, Err(ParseError::InvalidArgument(ref reason)) if reason == "invalid --slot: 65536")
    );
    let duplicate = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--slot",
        "2",
        "--slot",
        "3",
        "--value",
        "slot.bin",
        "--db",
        "run.db",
    ]));
    assert!(
        matches!(duplicate, Err(ParseError::InvalidArgument(ref reason)) if reason == "duplicate flag: --slot")
    );
}
