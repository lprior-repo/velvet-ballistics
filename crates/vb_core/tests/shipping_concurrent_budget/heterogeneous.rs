use vb_core::ids::StepIdx;
use vb_core::workflow::CompiledNode;

use super::fixtures::{action, end, finish, foreach, join};
use super::{K, n};

pub(super) fn graph() -> Vec<CompiledNode> {
    vec![
        n(
            0,
            K::TogetherConcurrentStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(6)]),
                join: StepIdx::new(8),
            },
            None,
        ),
        foreach(1, 4, 4, (2, 4)),
        action(2, 3),
        end(3, 1, 4),
        join(4, 1, 5),
        end(5, 0, 8),
        action(6, 7),
        end(7, 0, 8),
        join(8, 0, 9),
        finish(9),
    ]
}
