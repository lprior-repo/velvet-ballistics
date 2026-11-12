use super::fixtures::*;
use super::*;
use proptest::prelude::*;

#[derive(Debug, PartialEq, Eq)]
enum WidthOutcome {
    Width(usize),
    Overflow(usize),
    Unsupported {
        step: usize,
        primitive: &'static str,
    },
}

fn outcome(result: Result<usize, CompileError>) -> Result<WidthOutcome, String> {
    match result {
        Ok(width) => Ok(WidthOutcome::Width(width)),
        Err(CompileError::StepIndexOutOfRange { value }) => Ok(WidthOutcome::Overflow(value)),
        Err(CompileError::UnsupportedStepPrimitive { step, primitive }) => {
            Ok(WidthOutcome::Unsupported { step, primitive })
        }
        Err(other) => Err(format!("unexpected width error: {other:?}")),
    }
}

fn assert_width(body: &[StepAst], overhead: usize, expected: WidthOutcome) -> Result<(), String> {
    assert_eq!(outcome(body_width(body, overhead))?, expected);
    Ok(())
}

#[test]
fn nested_supported_containers_keep_literal_width() -> Result<(), String> {
    assert_width(&nested_body(), 5, WidthOutcome::Width(18))
}

#[test]
fn empty_root_preserves_maximum_overhead() -> Result<(), String> {
    assert_width(&[], usize::MAX, WidthOutcome::Width(usize::MAX))
}

#[test]
fn empty_containers_keep_each_original_overhead() -> Result<(), String> {
    let body = vec![
        for_each(Vec::new(), None),
        collect(Vec::new(), None, None),
        repeat(Vec::new(), 0),
        choose(Vec::new()),
    ];
    assert_width(&body, 0, WidthOutcome::Width(9))
}

#[test]
fn body_do_condition_remains_metadata_for_width() -> Result<(), String> {
    let mut action = do_step();
    action.condition = Some("true".to_owned());
    assert_width(&[action], 0, WidthOutcome::Width(1))
}

#[test]
fn every_unsupported_variant_keeps_exact_label_and_step() -> Result<(), String> {
    [
        (0, "save"),
        (1, "together"),
        (2, "reduce"),
        (3, "wait"),
        (4, "ask"),
        (5, "finish"),
    ]
    .into_iter()
    .try_for_each(|(kind, primitive)| {
        assert_width(
            &[step(unsupported(kind))],
            0,
            WidthOutcome::Unsupported { step: 0, primitive },
        )
    })
}

#[test]
fn child_error_precedes_parent_overflow_and_siblings_keep_order() -> Result<(), String> {
    let wait = || step(unsupported(3));
    let rejected = || WidthOutcome::Unsupported {
        step: 0,
        primitive: "wait",
    };
    assert_width(&[for_each(vec![wait()], None)], usize::MAX, rejected())?;
    assert_width(
        &[set(), wait()],
        usize::MAX,
        WidthOutcome::Overflow(usize::MAX),
    )?;
    assert_width(&[wait(), set()], usize::MAX, rejected())
}

#[test]
fn private_checked_cursor_reports_old_value_without_mutation() -> Result<(), String> {
    let mut index = usize::MAX;
    assert_eq!(
        outcome(advance_index(&mut index).map(|()| index))?,
        WidthOutcome::Overflow(usize::MAX)
    );
    assert_eq!(index, usize::MAX);
    Ok(())
}

#[test]
fn private_fold_reports_old_parent_width_without_mutation() -> Result<(), String> {
    let old = usize::MAX.saturating_sub(1);
    let mut stack = vec![Frame::body(&[], old), Frame::body(&[], 2)];
    assert_eq!(
        outcome(fold_frame(&mut stack).map(|_| 0))?,
        WidthOutcome::Overflow(old)
    );
    assert_eq!(stack.last().map(Frame::width), Some(old));
    Ok(())
}

#[test]
fn private_cursor_get_precedes_checked_increment() {
    let steps = [set()];
    let mut frame = Frame::Body {
        steps: &steps,
        index: usize::MAX,
        width: usize::MAX,
    };
    assert!(matches!(frame.advance(), Ok(Action::Fold)));
    assert!(matches!(
        frame,
        Frame::Body {
            index: usize::MAX,
            width: usize::MAX,
            ..
        }
    ));
}

#[test]
fn deep_repeat_width_uses_bounded_heap_walk() -> Result<(), String> {
    let body = deep_repeat_body();
    let result = outcome(body_width(&body, 0));
    release_repeat_body(body);
    assert_eq!(result?, WidthOutcome::Width(12289));
    Ok(())
}

proptest! {
    #[test]
    fn width_calculation_preserves_source_ast(body in arbitrary_body(), overhead in any::<usize>()) {
        let before = body.clone();
        outcome(body_width(&body, overhead))
            .map_err(proptest::test_runner::TestCaseError::fail)?;
        prop_assert_eq!(body, before);
    }
}
