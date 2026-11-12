use crate::{ChooseBranch, ScalarValue, StepAst, StepPrimitive};
use proptest::prelude::*;

pub(super) fn step(primitive: StepPrimitive) -> StepAst {
    StepAst {
        id: "width-step".to_owned(),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

pub(super) fn set() -> StepAst {
    step(StepPrimitive::Set {
        output: "v".to_owned(),
        value: "1".to_owned(),
    })
}

pub(super) fn do_step() -> StepAst {
    step(StepPrimitive::Do {
        action: "0".to_owned(),
        input: "0".to_owned(),
    })
}

pub(super) fn for_each(body: Vec<StepAst>, at_once: Option<u32>) -> StepAst {
    step(StepPrimitive::ForEach {
        variable: "v".to_owned(),
        input: "0".to_owned(),
        at_once,
        body,
    })
}

pub(super) fn collect(body: Vec<StepAst>, pages: Option<u32>, items: Option<u32>) -> StepAst {
    step(StepPrimitive::Collect {
        variable: "v".to_owned(),
        source: "0".to_owned(),
        pages,
        items,
        body,
    })
}

pub(super) fn repeat(body: Vec<StepAst>, max_attempts: u16) -> StepAst {
    step(StepPrimitive::Repeat { max_attempts, body })
}

pub(super) fn choose(branches: Vec<Vec<StepAst>>) -> StepAst {
    let branches = branches
        .into_iter()
        .map(|steps| ChooseBranch {
            when: "true".to_owned(),
            steps,
        })
        .collect();
    step(StepPrimitive::Choose {
        branches,
        otherwise: None,
    })
}

pub(super) fn unsupported(kind: u8) -> StepPrimitive {
    match kind {
        0 => StepPrimitive::Save {
            value: ScalarValue::Integer(0),
        },
        1 => StepPrimitive::Together {
            branches: Vec::new(),
        },
        2 => StepPrimitive::Aggregate {
            variable: "v".to_owned(),
            input: "0".to_owned(),
            initial: "0".to_owned(),
            body: Vec::new(),
        },
        3 => StepPrimitive::Wait {
            event: None,
            timeout: None,
        },
        4 => StepPrimitive::Ask {
            prompt: "prompt".to_owned(),
            timeout: None,
        },
        _ => StepPrimitive::Finish {
            result: ScalarValue::Integer(0),
        },
    }
}

fn loop_strategy(inner: BoxedStrategy<StepAst>) -> BoxedStrategy<StepAst> {
    let bodies = proptest::collection::vec(inner, 0..4);
    prop_oneof![
        (bodies.clone(), proptest::option::of(any::<u32>())).prop_map(|(b, a)| for_each(b, a)),
        (
            bodies.clone(),
            proptest::option::of(any::<u32>()),
            proptest::option::of(any::<u32>())
        )
            .prop_map(|(b, p, i)| collect(b, p, i)),
        (bodies, any::<u16>()).prop_map(|(b, n)| repeat(b, n)),
    ]
    .boxed()
}

fn container_strategy(inner: BoxedStrategy<StepAst>) -> BoxedStrategy<StepAst> {
    let branches = proptest::collection::vec(proptest::collection::vec(inner.clone(), 0..4), 0..3)
        .prop_map(choose);
    prop_oneof![loop_strategy(inner), branches].boxed()
}

pub(super) fn arbitrary_body() -> BoxedStrategy<Vec<StepAst>> {
    let leaf = prop_oneof![
        Just(set()),
        Just(do_step()),
        (0u8..6).prop_map(|kind| step(unsupported(kind))),
    ];
    let tree = leaf.prop_recursive(4, 64, 4, container_strategy);
    proptest::collection::vec(tree, 0..5).boxed()
}

pub(super) fn nested_body() -> Vec<StepAst> {
    let choice = choose(vec![vec![set(), do_step()], Vec::new()]);
    let retry = repeat(vec![choice, set()], 1);
    let collected = collect(vec![retry], None, None);
    vec![for_each(vec![collected], None), do_step()]
}

pub(super) fn deep_repeat_body() -> Vec<StepAst> {
    (0..4096).fold(vec![set()], |body, _| vec![repeat(body, 1)])
}

pub(super) fn release_repeat_body(mut body: Vec<StepAst>) {
    while let Some(step) = body.pop() {
        if let StepPrimitive::Repeat { body: children, .. } = step.primitive {
            body.extend(children);
        }
    }
}
