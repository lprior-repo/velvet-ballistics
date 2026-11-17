use super::{ReplayPushLimitDecision, classify_replay_push_len};
use crate::journal::EventReplayLimit;

#[test]
fn replay_push_classifier_checks_exact_capacity_boundary() -> Result<(), &'static str> {
    let limit = EventReplayLimit::new(3).ok_or("positive replay limit rejected")?;
    for (current_len, expected) in [
        (0, ReplayPushLimitDecision::Accept { observed: 1 }),
        (2, ReplayPushLimitDecision::Accept { observed: 3 }),
        (
            3,
            ReplayPushLimitDecision::TooMany {
                limit: 3,
                observed: 4,
            },
        ),
        (
            4,
            ReplayPushLimitDecision::TooMany {
                limit: 3,
                observed: 5,
            },
        ),
        (
            usize::MAX,
            ReplayPushLimitDecision::TooMany {
                limit: 3,
                observed: usize::MAX,
            },
        ),
    ] {
        assert_eq!(classify_replay_push_len(current_len, limit), expected);
    }
    Ok(())
}

#[test]
fn replay_push_classifier_rejects_overflow_even_at_maximum_limit() -> Result<(), &'static str> {
    let limit = EventReplayLimit::new(usize::MAX).ok_or("maximum replay limit rejected")?;
    assert_eq!(
        classify_replay_push_len(usize::MAX - 1, limit),
        ReplayPushLimitDecision::Accept {
            observed: usize::MAX
        }
    );
    assert_eq!(
        classify_replay_push_len(usize::MAX, limit),
        ReplayPushLimitDecision::TooMany {
            limit: usize::MAX,
            observed: usize::MAX
        }
    );
    Ok(())
}
