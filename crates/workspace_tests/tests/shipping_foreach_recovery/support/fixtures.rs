use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
};
use vb_core::{
    ActionId, CompiledNodeKind as K, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, Taint,
};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::FjallJournal;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const RUN: RunId = RunId::new(7_101);

pub(super) fn runtime(journal: Arc<FjallJournal>, step_budget_per_tick: u64) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

pub(super) fn multi_source(result: &str) -> String {
    format!("version: velvet-ballistics/v1\nname: shipping_foreach_recovery\nwhen: {{manual: {{}}}}\ninputs: {{items: {{is: list, of: any, max: 4}}}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      at_once: 2\n      steps:\n        - id: projected\n          set: {{output: projected, value: '$entry'}}\n  - id: check\n    set: {{output: check, value: '{result}'}}\n  - id: done\n    finish: {{result: '$check'}}\n")
}

pub(super) fn nested_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_nested_foreach\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: outer\n    for_each:\n      variable: row\n      input: $input.items\n      steps:\n        - id: inner\n          for_each:\n            variable: cell\n            input: '$row'\n            steps:\n              - id: projected\n                set: {output: projected, value: '$cell'}\n  - id: check\n    set: {output: check, value: 'count($step.outer) == 2 and count($step.outer.0) == 2 and $step.outer.0.0 == 4 and $step.outer.0.1 == 1 and count($step.outer.1) == 1 and $step.outer.1.0 == 9'}\n  - id: done\n    finish: {result: '$check'}\n".to_owned()
}

pub(super) fn reduce_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_reduce_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: folded\n    reduce:\n      variable: total\n      input: $input.items\n      initial: '10'\n      steps:\n        - id: update\n          set: {output: local, value: '$total + 1'}\n  - id: done\n    finish: {result: '$steps.folded'}\n".to_owned()
}

pub(super) fn together_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_together_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: group\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set: {output: left, value: 'count($input.items)'}\n        - label: right\n          steps:\n            - id: right_value\n              set: {output: right, value: '7'}\n  - id: check\n    set: {output: check, value: '$step.group.0 + $step.group.1'}\n  - id: done\n    finish: {result: '$check'}\n".to_owned()
}

pub(super) fn repeat_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_repeat_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: projected\n          set: {output: projected, value: 'count($input.items)'}\n  - id: check\n    set: {output: check, value: '$step.loop'}\n  - id: done\n    finish: {result: '$check'}\n".to_owned()
}

pub(super) fn object_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_object_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: projected\n          set: {output: projected, value: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n".to_owned()
}

pub(super) fn ask_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_ask_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: asked\n          ask:\n            prompt: '$entry'\n  - id: done\n    finish: {result: '$step.loop'}\n".to_owned()
}

pub(super) fn ask_timeout_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_ask_timeout_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: asked\n          ask: {prompt: '$entry', timeout: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n".to_owned()
}

pub(super) fn action_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_action_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: work\n          do: {action: '0', input: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n".to_owned()
}

pub(super) fn retry_action_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_action_retry_authority\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: work\n          try_again: {max_attempts: 3, delay: '0ms'}\n          do: {action: '0', input: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n".to_owned()
}

pub(super) fn wait_source() -> String {
    "version: velvet-ballistics/v1\nname: shipping_wait_recovery\nwhen: {manual: {}}\ninputs: {items: {is: list, of: any, max: 4}}\nsteps:\n  - id: loop\n    for_each:\n      variable: entry\n      input: $input.items\n      steps:\n        - id: waited\n          wait: {timeout: '$entry'}\n        - id: observed\n          set: {output: observed, value: '$entry'}\n  - id: done\n    finish: {result: '$step.loop'}\n".to_owned()
}

pub(super) fn action_contracts() -> TestResult<Box<[ActionContract]>> {
    let name = ActionName::new("shipping-action")?;
    Ok(Box::from([ActionContract {
        id: ActionId::new(0), name, input_slot_count: 1, output_slot_count: 1,
        max_input_bytes: 1024, max_output_bytes: 1024, timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure, side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe, required_capabilities: Box::new([]),
    }]))
}

pub(super) fn action_output(output: SlotIdx, value: SlotValue) -> TestResult<ActionOutputReady> {
    let encoded = postcard::to_allocvec(&value)?;
    Ok(ActionOutputReady { output_slot: output, value, taint: Taint::Clean, encoded_len: u32::try_from(encoded.len())? })
}

pub(super) fn body_step(source: &str) -> TestResult<StepIdx> {
    control_step(source, |kind| matches!(kind, K::ForEachNext { .. }))
}

pub(super) fn control_step(source: &str, predicate: impl Fn(&K) -> bool) -> TestResult<StepIdx> {
    let workflow = vb_compile::compile_workflow(source.as_bytes())?;
    crate::support::cut::step_with(&workflow, predicate)
}
