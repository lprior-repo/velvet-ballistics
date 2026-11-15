use super::*;

#[test]
fn parse_answer_rejects_legacy_step_and_accepts_canonical_slot_selector() {
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
    assert_eq!(
        step,
        Err(ParseError::UnknownFlag {
            command: "answer",
            flag: "--step".into(),
        })
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
fn parse_answer_rejects_legacy_value_file_flag() {
    let legacy_value = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "41",
        "--slot",
        "2",
        "--value",
        "a.bin",
        "--value-file",
        "b.bin",
        "--db",
        "run.db",
    ]));
    assert_eq!(
        legacy_value,
        Err(ParseError::UnknownFlag {
            command: "answer",
            flag: "--value-file".into(),
        })
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
