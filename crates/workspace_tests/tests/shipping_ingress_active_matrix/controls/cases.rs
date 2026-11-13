use vb_core::SlotValue;

const HEADER: &str = "version: velvet-ballistics/v1\nname: ingress_active_control\nwhen: {manual: {}}\nvars: {items: [1, 2, 3]}\n";

pub(super) struct ControlCase {
    pub(super) name: &'static str,
    pub(super) body: &'static str,
    pub(super) source: String,
    pub(super) outputs: Vec<(&'static str, Vec<SlotValue>)>,
}

pub(super) fn cases() -> Vec<ControlCase> {
    vec![
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    for_each:\n      variable: entry\n      input: '$vars.items'\n      at_once: 1\n      steps:\n        - id: body\n          set: {{output: seen, value: '$entry'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![(
                "body",
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
            )],
        },
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    reduce:\n      variable: total\n      input: '$vars.items'\n      initial: '0'\n      steps:\n        - id: body\n          set: {{output: running, value: '$total + 1'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![(
                "body",
                vec![
                    SlotValue::I64(0),
                    SlotValue::I64(1),
                    SlotValue::I64(2),
                    SlotValue::I64(3),
                    SlotValue::I64(3),
                ],
            )],
        },
        ControlCase {
            name: "outer",
            body: "left_value",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set: {{output: left, value: '7'}}\n        - label: right\n          steps:\n            - id: right_value\n              set: {{output: right, value: '9'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![
                ("left_value", vec![SlotValue::I64(7)]),
                ("right_value", vec![SlotValue::I64(9)]),
            ],
        },
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: body\n          set: {{output: tick, value: '7'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            outputs: vec![("body", vec![SlotValue::I64(7), SlotValue::I64(7)])],
        },
        ControlCase {
            name: "outer",
            body: "body",
            source: format!(
                "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    collect:\n      variable: batch\n      source: '$vars.items'\n      pages: 2\n      items: 2\n      steps:\n        - id: body\n          set: {{output: copied, value: '$batch'}}\n  - id: total\n    set: {{output: total, value: 'sum($step.outer)'}}\n  - id: done\n    finish: {{result: '$step.total'}}\n"
            ),
            outputs: vec![(
                "body",
                vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
            )],
        },
    ]
}
