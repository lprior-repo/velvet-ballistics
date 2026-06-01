#![forbid(unsafe_code)]
//! Shared helper functions for workflow analysis.

use vb_core::CompiledNodeKind;

/// Returns a static label string for a compiled node kind.
pub(crate) fn node_kind_label(kind: &CompiledNodeKind) -> &'static str {
    match kind {
        CompiledNodeKind::Nop => "nop",
        CompiledNodeKind::SetConst { .. } => "set_const",
        CompiledNodeKind::Copy { .. } => "copy",
        CompiledNodeKind::EvalExpr { .. } => "eval_expr",
        CompiledNodeKind::BuildObject { .. } => "build_object",
        CompiledNodeKind::BuildList { .. } => "build_list",
        CompiledNodeKind::Do { .. } => "do",
        CompiledNodeKind::Choose { .. } => "choose",
        CompiledNodeKind::ChooseSlot { .. } => "choose_slot",
        CompiledNodeKind::ForEachStart { .. } => "for_each_start",
        CompiledNodeKind::ForEachNext { .. } => "for_each_next",
        CompiledNodeKind::ForEachJoin { .. } => "for_each_join",
        CompiledNodeKind::TogetherStart { .. } => "together_start",
        CompiledNodeKind::TogetherBranch { .. } => "together_branch",
        CompiledNodeKind::TogetherJoin { .. } => "together_join",
        CompiledNodeKind::CollectStart { .. } => "collect_start",
        CompiledNodeKind::CollectPage { .. } => "collect_page",
        CompiledNodeKind::CollectNext { .. } => "collect_next",
        CompiledNodeKind::CollectFinish { .. } => "collect_finish",
        CompiledNodeKind::ReduceStart { .. } => "reduce_start",
        CompiledNodeKind::ReduceNext { .. } => "reduce_next",
        CompiledNodeKind::ReduceFinish { .. } => "reduce_finish",
        CompiledNodeKind::RepeatStart { .. } => "repeat_start",
        CompiledNodeKind::RepeatAttempt { .. } => "repeat_attempt",
        CompiledNodeKind::RepeatCheck { .. } => "repeat_check",
        CompiledNodeKind::RepeatFinish { .. } => "repeat_finish",
        CompiledNodeKind::WaitUntil { .. } => "wait_until",
        CompiledNodeKind::WaitEvent { .. } => "wait_event",
        CompiledNodeKind::Ask { .. } => "ask",
        CompiledNodeKind::AskResume { .. } => "ask_resume",
        CompiledNodeKind::RetryCheck { .. } => "retry_check",
        CompiledNodeKind::ErrorHandler { .. } => "error_handler",
        CompiledNodeKind::Jump { .. } => "jump",
        CompiledNodeKind::Finish { .. } => "finish",
        _ => "unknown",
    }
}

/// Saturating add that returns the new value, used instead of checked_add +
/// unwrap/or pattern.
pub(crate) fn saturating_add(a: usize, b: usize) -> usize {
    a.saturating_add(b)
}