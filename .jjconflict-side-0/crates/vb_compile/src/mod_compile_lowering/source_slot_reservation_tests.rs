//! F2 allocator and authored-slot reservation regression tests.

use super::*;
use vb_core::SlotIdx;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn source(steps: &str) -> Result<WorkflowSource, crate::YamlError> {
    crate::parse_workflow_source(&format!(
        "version: velvet-ballistics/v1\nname: reservation\nwhen:\n  manual: {{}}\nsteps:\n{steps}"
    ))
}

#[test]
fn allocations_and_reservations_are_monotonic() -> TestResult {
    let mut builder = SlotCompiler::new();
    assert_eq!(builder.allocate_slot()?, SlotIdx::new(0));
    builder.reserve_slots(8)?;
    builder.reserve_slots(0)?;
    builder.reserve_slots(3)?;
    assert_eq!(builder.slot_count()?, 8);
    assert_eq!(builder.allocate_slot()?, SlotIdx::new(8));
    builder.record_slot(SlotIdx::new(20));
    assert_eq!(builder.allocate_slot()?, SlotIdx::new(21));
    assert_eq!(builder.slot_count()?, 22);
    Ok(())
}

#[test]
fn allocation_checks_every_representable_count_before_mutation() -> TestResult {
    assert_eq!(MAX_SOURCE_STEPS, usize::from(u16::MAX));
    for count in 0..=u16::MAX {
        let mut builder = SlotCompiler::new();
        builder.reserve_slots(count)?;
        let before = builder.max_slot;
        let result = builder.allocate_slot();
        if let Some(next) = count.checked_add(1) {
            assert_eq!(result?, SlotIdx::new(count));
            assert_eq!(builder.slot_count()?, next);
        } else {
            assert!(matches!(
                result,
                Err(CompileError::SlotIndexOutOfRange { .. })
            ));
            assert_eq!(builder.max_slot, before);
        }
    }
    Ok(())
}

#[test]
fn invalid_recorded_slot_rejects_without_mutation() {
    let mut builder = SlotCompiler::new();
    builder.record_slot(SlotIdx::new(u16::MAX));
    let before = builder.max_slot;
    assert!(builder.allocate_slot().is_err());
    assert_eq!(builder.max_slot, before);
    assert!(builder.reserve_slots(1).is_err());
    assert_eq!(builder.max_slot, before);
    assert_eq!(builder.reserve_slots(0), Ok(()));
    assert_eq!(builder.max_slot, before);
}

#[test]
fn positional_outputs_preserve_holes_and_ignore_literals() -> TestResult {
    let source = source(
        "  - id: first\n    set: {output: answer, value: \"65535\"}\n  - id: pause\n    wait: {timeout: \"0\"}\n  - id: later\n    set: {output: later_value, value: \"900000\"}\n  - id: done\n    finish: {result: answer}\n",
    )?;
    let mut builder = SlotCompiler::new();
    reserve_source_slots(&source, &mut builder)?;
    assert_eq!(builder.slot_count()?, 3);
    assert_eq!(builder.allocate_slot()?, SlotIdx::new(3));
    Ok(())
}

#[test]
fn numeric_read_fields_reserve_but_non_slot_numbers_do_not() -> TestResult {
    let cases = [
        ("    do: {action: \"65535\", input: \"7\"}\n", 8),
        (
            "    for_each: {variable: item, input: \"8\", at_once: 9000, steps: []}\n",
            9,
        ),
        (
            "    collect: {variable: page, source: \"9\", pages: 9000, items: 8000, steps: []}\n",
            10,
        ),
        (
            "    reduce: {variable: total, input: \"10\", initial: \"65535\", steps: []}\n",
            11,
        ),
        ("    wait: {event: \"11\", timeout: \"12\"}\n", 13),
        ("    ask: {prompt: \"13\", timeout: \"14\"}\n", 15),
        ("    finish: {result: 15}\n", 16),
        ("    choose: {branches: [{when: \"16\", steps: []}]}\n", 17),
        ("    finish: {result: \"65535\"}\n", 0),
        ("    repeat: {max_attempts: 65535, steps: []}\n", 0),
    ];
    for (primitive, count) in cases {
        let source = source(&format!("  - id: probe\n{primitive}"))?;
        let mut builder = SlotCompiler::new();
        reserve_source_slots(&source, &mut builder)?;
        assert_eq!(builder.slot_count()?, count, "{primitive}");
    }
    Ok(())
}

#[test]
fn nested_branch_and_loop_reads_are_reserved_before_scratch() -> TestResult {
    let source = source(
        "  - id: group\n    together:\n      branches:\n        - label: branch\n          steps:\n            - id: retry\n              repeat:\n                max_attempts: 2\n                steps:\n                  - id: choice\n                    choose:\n                      branches:\n                        - when: \"40\"\n                          steps:\n                            - id: action\n                              do: {action: \"65535\", input: \"63\"}\n  - id: done\n    finish: {result: 0}\n",
    )?;
    let mut builder = SlotCompiler::new();
    reserve_source_slots(&source, &mut builder)?;
    assert_eq!(builder.allocate_slot()?, SlotIdx::new(64));
    Ok(())
}

#[test]
fn each_scoped_body_is_walked_without_reserving_its_set_literals() -> TestResult {
    let cases = [
        "for_each: {variable: item, input: \"0\", steps: BODY}",
        "collect: {variable: page, source: \"0\", steps: BODY}",
        "reduce: {variable: total, input: \"0\", initial: \"65535\", steps: BODY}",
        "repeat: {max_attempts: 2, steps: BODY}",
        "together: {branches: [{label: branch, steps: BODY}]}",
        "choose: {branches: [{when: \"0\", steps: BODY}]}",
    ];
    for primitive in cases {
        let body = "[{id: literal, set: {output: ignored, value: \"65535\"}}, {id: read, do: {action: \"65535\", input: \"31\"}}]";
        let primitive = primitive.replace("BODY", body);
        let source = source(&format!("  - id: scope\n    {primitive}\n"))?;
        let mut builder = SlotCompiler::new();
        reserve_source_slots(&source, &mut builder)?;
        assert_eq!(builder.allocate_slot()?, SlotIdx::new(32));
    }
    Ok(())
}

#[test]
fn invalid_late_reference_leaves_previous_reservation_unchanged() -> TestResult {
    for value in ["-1", "65535", "65536"] {
        let source = source(&format!(
            "  - id: first\n    set: {{output: answer, value: \"42\"}}\n  - id: external\n    ask: {{prompt: \"{value}\"}}\n"
        ))?;
        let mut builder = SlotCompiler::new();
        builder.reserve_slots(3)?;
        assert!(matches!(
            reserve_source_slots(&source, &mut builder),
            Err(CompileError::SlotIndexOutOfRange { .. })
        ));
        assert_eq!(builder.slot_count()?, 3);
    }
    Ok(())
}

#[test]
fn highest_valid_reference_is_reserved_without_wrapping() -> TestResult {
    let source = source("  - id: done\n    finish: {result: 65534}\n")?;
    let mut builder = SlotCompiler::new();
    reserve_source_slots(&source, &mut builder)?;
    assert_eq!(builder.slot_count()?, u16::MAX);
    assert!(builder.allocate_slot().is_err());
    assert_eq!(builder.slot_count()?, u16::MAX);
    Ok(())
}
