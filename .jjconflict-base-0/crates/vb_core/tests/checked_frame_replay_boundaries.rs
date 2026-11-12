#[path = "checked_frame_replay_boundaries/support.rs"]
mod support;
use support::{TestResult, conditions, dispatch, frame, plan};
use vb_core::replay::ReplayError;
use vb_core::{
    BlobId, CoreError, FiniteF64, ListId, ObjectId, SlotIdx, SlotValue, StepIdx, SymbolId, Taint,
};

#[test]
fn characterize_public_slot_reads_preserve_all_payloads_taints_and_exact_errors() -> TestResult {
    let mut run = frame(2, 2, 19)?;
    let before = run.clone();
    assert_eq!(
        run.read_slot(SlotIdx::new(0)),
        Err(CoreError::SlotUninitialized {
            slot: SlotIdx::new(0)
        })
    );
    for slot in [SlotIdx::new(2), SlotIdx::MAX] {
        assert_eq!(
            run.read_slot(slot),
            Err(CoreError::SlotOutOfBounds { slot })
        );
    }
    assert_eq!(run, before);
    let values = [
        SlotValue::Null,
        SlotValue::Bool(false),
        SlotValue::Bool(true),
        SlotValue::I64(i64::MIN),
        SlotValue::I64(i64::MAX),
        SlotValue::F64(FiniteF64::from_i64(-7)),
        SlotValue::Symbol(SymbolId::new(u32::MAX)),
        SlotValue::List(ListId::new(u32::MAX)),
        SlotValue::Object(ObjectId::new(u32::MAX)),
        SlotValue::Blob(BlobId::new(u64::MAX)),
    ];
    for value in values {
        for taint in [
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret,
            Taint::Random,
            Taint::TimeDependent,
        ] {
            run.write_slot_with_taint(SlotIdx::new(0), value, taint)?;
            let before = run.clone();
            assert_eq!(run.read_slot(SlotIdx::new(0)), Ok(&value));
            assert_eq!(run.read_taint(SlotIdx::new(0)), Ok(taint));
            assert_eq!(run, before);
        }
    }
    Ok(())
}

#[test]
fn characterize_public_pc_and_counter_changes_only_their_own_fields() -> TestResult {
    for target in [
        StepIdx::ZERO,
        StepIdx::new(3),
        StepIdx::new(4),
        StepIdx::MAX,
    ] {
        let mut run = frame(4, 2, 19)?;
        conditions(&mut run, Some(SlotValue::Bool(true)), Some(SlotValue::Null))?;
        let mut expected = run.checkpoint_v1();
        let expected_result = match target.get() {
            0 | 3 => {
                expected.pc = target;
                Ok(())
            }
            _ => Err(CoreError::InvalidProgramCounter { step: target }),
        };
        assert_eq!(run.set_pc(target), expected_result);
        assert_eq!(run.checkpoint_v1(), expected);
    }
    for (initial, expected_counter) in [
        (0, Some(1)),
        (u64::MAX.saturating_sub(1), Some(u64::MAX)),
        (u64::MAX, None),
    ] {
        let mut run = frame(4, 2, initial)?;
        conditions(&mut run, Some(SlotValue::Bool(true)), Some(SlotValue::Null))?;
        let mut expected = run.checkpoint_v1();
        let expected_result = if let Some(counter) = expected_counter {
            expected.executed = counter;
            Ok(())
        } else {
            Err(CoreError::StepCounterOverflow)
        };
        assert_eq!(run.increment_executed(), expected_result);
        assert_eq!(run.checkpoint_v1(), expected);
    }
    Ok(())
}

#[test]
fn characterize_choice_pc_commits_before_counter_overflow_for_branches_and_fallback() -> TestResult
{
    let plan = plan(true)?;
    for (a, b, target) in [(true, false, 1), (false, true, 2), (false, false, 3)] {
        for (initial, counter) in [
            (0, Some(1)),
            (u64::MAX.saturating_sub(1), Some(u64::MAX)),
            (u64::MAX, None),
        ] {
            let mut run = frame(4, 2, initial)?;
            conditions(&mut run, Some(SlotValue::Bool(a)), Some(SlotValue::Bool(b)))?;
            let mut expected = run.checkpoint_v1();
            expected.pc = StepIdx::new(target);
            let result = if let Some(counter) = counter {
                expected.executed = counter;
                Ok(StepIdx::new(target))
            } else {
                Err(ReplayError::Internal {
                    reason: "executed counter overflow",
                })
            };
            assert_eq!(dispatch(&plan, &mut run)?, result);
            assert_eq!(run.checkpoint_v1(), expected);
        }
    }
    Ok(())
}

#[test]
fn characterize_invalid_choice_target_precedes_counter_overflow_without_any_writes() -> TestResult {
    let plan = plan(true)?;
    for (a, b, target) in [(true, false, 1), (false, true, 2), (false, false, 3)] {
        for initial in [0, u64::MAX] {
            let mut run = frame(target, 2, initial)?;
            conditions(&mut run, Some(SlotValue::Bool(a)), Some(SlotValue::Bool(b)))?;
            let before = run.clone();
            assert_eq!(
                dispatch(&plan, &mut run)?,
                Err(ReplayError::StepNotFound {
                    step: StepIdx::new(target)
                })
            );
            assert_eq!(run, before);
        }
    }
    Ok(())
}

#[test]
fn characterize_choice_read_and_no_match_failures_preserve_prestate() -> TestResult {
    let no_match = ReplayError::Internal {
        reason: "choose_slot no branch matched and no otherwise",
    };
    let wrong_type = ReplayError::Internal {
        reason: "choose_slot condition is not boolean",
    };
    let cases = [
        (
            2,
            None,
            Some(SlotValue::Bool(true)),
            true,
            ReplayError::SlotNotAvailable {
                slot: SlotIdx::new(0),
            },
        ),
        (
            2,
            Some(SlotValue::Bool(false)),
            None,
            true,
            ReplayError::SlotNotAvailable {
                slot: SlotIdx::new(1),
            },
        ),
        (
            1,
            Some(SlotValue::Bool(false)),
            None,
            true,
            ReplayError::SlotNotAvailable {
                slot: SlotIdx::new(1),
            },
        ),
        (
            0,
            None,
            None,
            true,
            ReplayError::SlotNotAvailable {
                slot: SlotIdx::new(0),
            },
        ),
        (
            2,
            Some(SlotValue::I64(7)),
            Some(SlotValue::Bool(true)),
            true,
            wrong_type.clone(),
        ),
        (
            2,
            Some(SlotValue::Bool(false)),
            Some(SlotValue::Null),
            true,
            wrong_type,
        ),
        (
            2,
            Some(SlotValue::Bool(false)),
            Some(SlotValue::Bool(false)),
            false,
            no_match,
        ),
    ];
    for (slots, a, b, otherwise, error) in cases {
        let plan = plan(otherwise)?;
        let mut run = frame(4, slots, u64::MAX)?;
        conditions(&mut run, a, b)?;
        let before = run.clone();
        assert_eq!(dispatch(&plan, &mut run)?, Err(error));
        assert_eq!(run, before);
    }
    let plan = plan(true)?;
    let mut run = frame(4, 1, 0)?;
    conditions(&mut run, Some(SlotValue::Bool(true)), None)?;
    let mut expected = run.checkpoint_v1();
    expected.pc = StepIdx::new(1);
    expected.executed = 1;
    assert_eq!(dispatch(&plan, &mut run)?, Ok(StepIdx::new(1)));
    assert_eq!(run.checkpoint_v1(), expected);
    Ok(())
}
