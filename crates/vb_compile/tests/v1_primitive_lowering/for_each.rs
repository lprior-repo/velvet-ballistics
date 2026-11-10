use super::*;
use vb_core::{ConstIdx, ExprIdx};

type Expected = (Vec<CompiledNode>, Vec<ConstValue>, u16);

pub(super) fn expected(preamble: bool, values: &[i64]) -> Result<Expected, String> {
    let base = u16::from(preamble);
    let add = |raw: u16| raw.checked_add(base).ok_or("offset".to_string());
    let step = |raw| add(raw).map(StepIdx::new);
    let slot = |raw| add(raw).map(SlotIdx::new);
    let count = u16::try_from(values.len()).map_err(|e| e.to_string())?;
    let append = count.checked_add(3).ok_or("append")?;
    let next = append.checked_add(1).ok_or("next")?;
    let join = next.checked_add(1).ok_or("join")?;
    let finish = join.checked_add(1).ok_or("finish")?;
    let mut nodes = Vec::new();
    let mut constants = Vec::new();
    if preamble {
        constants.push(ConstValue::I64(1));
        nodes.push(node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            Some(step(0)?),
        ));
    }
    constants.push(ConstValue::Bool(false));
    nodes.extend([
        node(
            step(0)?,
            CompiledNodeKind::BuildList {
                items: Box::new([]),
            },
            Some(slot(1)?),
            Some(step(1)?),
        ),
        node(
            step(1)?,
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(0),
            },
            Some(slot(1)?),
            Some(step(2)?),
        ),
        node(
            step(2)?,
            CompiledNodeKind::ForEachStart {
                input: slot(0)?,
                item_slot: slot(3)?,
                limit: 2,
                body: step(3)?,
                done: step(join)?,
            },
            Some(slot(2)?),
            None,
        ),
    ]);
    for (index, value) in values.iter().enumerate() {
        let index = u16::try_from(index).map_err(|e| e.to_string())?;
        let constant = index
            .checked_add(1)
            .and_then(|i| i.checked_add(base))
            .ok_or("constant")?;
        let id = index.checked_add(3).ok_or("body")?;
        let output = index.checked_add(5).ok_or("output")?;
        constants.push(ConstValue::I64(*value));
        nodes.push(node(
            step(id)?,
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(constant),
            },
            Some(slot(output)?),
            Some(step(id.checked_add(1).ok_or("body next")?)?),
        ));
    }
    nodes.extend([
        node(
            step(append)?,
            CompiledNodeKind::EvalExpr {
                expr: ExprIdx::new(1),
            },
            Some(slot(1)?),
            Some(step(next)?),
        ),
        node(
            step(next)?,
            CompiledNodeKind::ForEachNext {
                iterator_slot: slot(2)?,
                body: step(3)?,
                done: step(join)?,
            },
            Some(slot(3)?),
            None,
        ),
        node(
            step(join)?,
            CompiledNodeKind::ForEachJoin { output: slot(1)? },
            Some(slot(4)?),
            Some(step(finish)?),
        ),
        node(
            step(finish)?,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ]);
    Ok((
        nodes,
        constants,
        count
            .checked_add(5)
            .and_then(|n| n.checked_add(base))
            .ok_or("slots")?,
    ))
}

pub(super) const PRIMITIVE_CASES: &[PrimitiveCase] = &[
    PrimitiveCase {
        name: "for_each",
        yaml_steps: "  - id: loop\n    for_each:\n      variable: entry\n      input: \"$input.items\"\n      at_once: 1\n      steps:\n        - id: capture\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: $input.items\n",
        expected_kinds: FOREACH_KINDS,
        expected_slot_count: 6,
    },
    PrimitiveCase {
        name: "together",
        yaml_steps: "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_set\n              set:\n                output: left\n                value: \"1\"\n        - label: right\n          steps:\n            - id: right_set\n              set:\n                output: right\n                value: \"2\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: TOGETHER_KINDS,
        expected_slot_count: 5,
    },
    PrimitiveCase {
        name: "collect",
        yaml_steps: "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 3\n      items: 5\n      steps:\n        - id: remember_page\n          set:\n            output: page_seen\n            value: \"7\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: COLLECT_KINDS,
        expected_slot_count: 4,
    },
    PrimitiveCase {
        name: "reduce",
        yaml_steps: "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: add_one
          set:\n            output: acc_out\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: REDUCE_KINDS,
        expected_slot_count: 5,
    },
    PrimitiveCase {
        name: "repeat",
        yaml_steps: "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: REPEAT_KINDS,
        expected_slot_count: 4,
    },
    PrimitiveCase {
        name: "wait",
        yaml_steps: "  - id: wait_for_event\n    wait:\n      event: \"0\"\n      timeout: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: WAIT_EVENT_KINDS,
        expected_slot_count: 2,
    },
    PrimitiveCase {
        name: "ask",
        yaml_steps: "  - id: ask_human\n    ask:\n      prompt: \"0\"\n      timeout: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
        expected_kinds: ASK_KINDS,
        expected_slot_count: 3,
    },
];

pub(super) fn node_kind_names(nodes: &[vb_core::CompiledNode]) -> Vec<&'static str> {
    nodes
        .iter()
        .map(|node| node_kind_name(&node.kind))
        .collect()
}

pub(super) fn node_kind_name(kind: &CompiledNodeKind) -> &'static str {
    match kind {
        CompiledNodeKind::Nop => "Nop",
        CompiledNodeKind::SetConst { .. } => "SetConst",
        CompiledNodeKind::Copy { .. } => "Copy",
        CompiledNodeKind::EvalExpr { .. } => "EvalExpr",
        CompiledNodeKind::BuildObject { .. } => "BuildObject",
        CompiledNodeKind::BuildList { .. } => "BuildList",
        CompiledNodeKind::Do { .. } => "Do",
        CompiledNodeKind::Choose { .. } => "Choose",
        CompiledNodeKind::ChooseSlot { .. } => "ChooseSlot",
        CompiledNodeKind::ForEachStart { .. } => "ForEachStart",
        CompiledNodeKind::ForEachNext { .. } => "ForEachNext",
        CompiledNodeKind::ForEachJoin { .. } => "ForEachJoin",
        CompiledNodeKind::TogetherStart { .. } => "TogetherStart",
        CompiledNodeKind::TogetherBranch { .. } => "TogetherBranch",
        CompiledNodeKind::TogetherJoin { .. } => "TogetherJoin",
        CompiledNodeKind::CollectStart { .. } => "CollectStart",
        CompiledNodeKind::CollectPage { .. } => "CollectPage",
        CompiledNodeKind::CollectNext { .. } => "CollectNext",
        CompiledNodeKind::CollectFinish { .. } => "CollectFinish",
        CompiledNodeKind::ReduceStart { .. } => "ReduceStart",
        CompiledNodeKind::ReduceNext { .. } => "ReduceNext",
        CompiledNodeKind::ReduceFinish { .. } => "ReduceFinish",
        CompiledNodeKind::RepeatStart { .. } => "RepeatStart",
        CompiledNodeKind::RepeatAttempt { .. } => "RepeatAttempt",
        CompiledNodeKind::RepeatCheck { .. } => "RepeatCheck",
        CompiledNodeKind::RepeatFinish { .. } => "RepeatFinish",
        CompiledNodeKind::WaitUntil { .. } => "WaitUntil",
        CompiledNodeKind::WaitEvent { .. } => "WaitEvent",
        CompiledNodeKind::Ask { .. } => "Ask",
        CompiledNodeKind::AskResume { .. } => "AskResume",
        CompiledNodeKind::RetryCheck { .. } => "RetryCheck",
        CompiledNodeKind::ErrorHandler { .. } => "ErrorHandler",
        CompiledNodeKind::Jump { .. } => "Jump",
        CompiledNodeKind::Finish { .. } => "Finish",
        _ => "Unknown",
    }
}
