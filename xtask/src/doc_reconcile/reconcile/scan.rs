use crate::doc_reconcile::{
    ContradictionReport, DocReconcileError, PatchEdit, PreservedNonGoal, ResolvedNode, StalePhrase,
};

use super::text::contains_case_insensitive;

struct StalePattern {
    node: ResolvedNode,
    phrase: &'static str,
    stale: StalePhrase,
    edit: PatchEdit,
    no_join: bool,
    write_slot_only: bool,
    matcher: StaleMatcher,
}

#[derive(Clone, Copy)]
enum StaleMatcher {
    AlwaysClean,
    Phrase,
    WriteSlotOnly,
    FinishSignalWithoutTaint,
}

const STALE_PATTERNS: &[StalePattern] = &[
    stale(
        ResolvedNode::EvalExpr,
        "Always Clean",
        StalePhrase::EvalExprAlwaysClean,
        PatchEdit::EvalExprJoin,
        false,
        false,
        StaleMatcher::AlwaysClean,
    ),
    stale(
        ResolvedNode::EvalExpr,
        "No taint join",
        StalePhrase::EvalExprNoOperandJoin,
        PatchEdit::EvalExprJoin,
        true,
        false,
        StaleMatcher::Phrase,
    ),
    stale(
        ResolvedNode::EvalExpr,
        "write_slot",
        StalePhrase::WriteSlotOnly,
        PatchEdit::EvalExprJoin,
        false,
        true,
        StaleMatcher::WriteSlotOnly,
    ),
    stale(
        ResolvedNode::BuildObject,
        "Always Clean",
        StalePhrase::BuildObjectAlwaysClean,
        PatchEdit::BuildObjectJoin,
        false,
        false,
        StaleMatcher::AlwaysClean,
    ),
    stale(
        ResolvedNode::BuildObject,
        "no join of field taints",
        StalePhrase::BuildObjectNoFieldJoin,
        PatchEdit::BuildObjectJoin,
        true,
        false,
        StaleMatcher::Phrase,
    ),
    stale(
        ResolvedNode::BuildObject,
        "write_slot",
        StalePhrase::WriteSlotOnly,
        PatchEdit::BuildObjectJoin,
        false,
        true,
        StaleMatcher::WriteSlotOnly,
    ),
    stale(
        ResolvedNode::BuildList,
        "Always Clean",
        StalePhrase::BuildListAlwaysClean,
        PatchEdit::BuildListJoin,
        false,
        false,
        StaleMatcher::AlwaysClean,
    ),
    stale(
        ResolvedNode::BuildList,
        "no join of item taints",
        StalePhrase::BuildListNoItemJoin,
        PatchEdit::BuildListJoin,
        true,
        false,
        StaleMatcher::Phrase,
    ),
    stale(
        ResolvedNode::BuildList,
        "write_slot",
        StalePhrase::WriteSlotOnly,
        PatchEdit::BuildListJoin,
        false,
        true,
        StaleMatcher::WriteSlotOnly,
    ),
    stale(
        ResolvedNode::Finish,
        "Finished(SlotValue)",
        StalePhrase::WriteSlotOnly,
        PatchEdit::FinishCarriesTaint,
        false,
        true,
        StaleMatcher::FinishSignalWithoutTaint,
    ),
];

const SCANNED_NODES: &[ResolvedNode] = &[
    ResolvedNode::EvalExpr,
    ResolvedNode::BuildObject,
    ResolvedNode::BuildList,
    ResolvedNode::Finish,
];

const fn stale(
    node: ResolvedNode,
    phrase: &'static str,
    stale: StalePhrase,
    edit: PatchEdit,
    no_join: bool,
    write_slot_only: bool,
    matcher: StaleMatcher,
) -> StalePattern {
    StalePattern {
        node,
        phrase,
        stale,
        edit,
        no_join,
        write_slot_only,
        matcher,
    }
}

pub(super) fn collect_contradictions(text: &str) -> ContradictionReport {
    let mut report = ContradictionReport {
        stale_clean_only: Vec::new(),
        no_join_claims: Vec::new(),
        write_slot_only_claims: Vec::new(),
        scanned_nodes: SCANNED_NODES.to_vec(),
    };
    for pattern in STALE_PATTERNS {
        if pattern_matches(pattern, text) {
            report.stale_clean_only.push(pattern.stale);
            if pattern.no_join {
                report.no_join_claims.push(pattern.stale);
            }
            if pattern.write_slot_only {
                report.write_slot_only_claims.push(pattern.stale);
            }
        }
    }
    report
}

pub(super) fn first_stale_error(text: &str) -> Option<DocReconcileError> {
    if contains_finish_rejection(text) {
        return Some(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "rejects finish taint".to_owned(),
        });
    }
    STALE_PATTERNS.iter().find_map(|pattern| {
        pattern_matches(pattern, text).then(|| DocReconcileError::StaleCleanOnlyTaintText {
            node: pattern.node,
            phrase: pattern.phrase.to_owned(),
        })
    })
}

pub(super) fn edits_for(text: &str) -> Vec<PatchEdit> {
    let mut edits = Vec::new();
    for pattern in STALE_PATTERNS {
        if pattern_matches(pattern, text) {
            push_unique_edit(&mut edits, pattern.edit);
        }
    }
    if finish_edit_needed(text) {
        push_unique_edit(&mut edits, PatchEdit::FinishCarriesTaint);
    }
    edits
}

pub(super) fn preserved_non_goals(text: &str) -> Vec<PreservedNonGoal> {
    if text.contains("does not track control-flow taint") {
        vec![PreservedNonGoal::ControlFlowTaintV1NonGoal]
    } else {
        Vec::new()
    }
}

fn push_unique_edit(edits: &mut Vec<PatchEdit>, edit: PatchEdit) {
    if !edits.contains(&edit) {
        edits.push(edit);
    }
}

fn pattern_matches(pattern: &StalePattern, text: &str) -> bool {
    match pattern.matcher {
        StaleMatcher::AlwaysClean => segment_contains_node_and(text, pattern.node, |segment| {
            segment.contains("always clean")
        }),
        StaleMatcher::Phrase => segment_contains_node_and(text, pattern.node, |segment| {
            segment.contains(&pattern.phrase.to_ascii_lowercase())
        }),
        StaleMatcher::WriteSlotOnly => {
            segment_contains_node_and(text, pattern.node, contains_write_slot_only_claim)
        }
        StaleMatcher::FinishSignalWithoutTaint => text.contains(pattern.phrase),
    }
}

fn segment_contains_node_and(
    text: &str,
    node: ResolvedNode,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    let node = node_label(node).to_ascii_lowercase();
    text.split(['\n', '.'])
        .map(str::to_ascii_lowercase)
        .any(|segment| segment.contains(&node) && predicate(&segment))
}

fn node_label(node: ResolvedNode) -> &'static str {
    match node {
        ResolvedNode::EvalExpr => "EvalExpr",
        ResolvedNode::BuildObject => "BuildObject",
        ResolvedNode::BuildList => "BuildList",
        ResolvedNode::Finish => "Finish",
    }
}

fn contains_write_slot_only_claim(segment: &str) -> bool {
    segment.contains("write_slot") && segment.contains("not write_slot_with_taint")
}

fn contains_finish_rejection(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("no rejection") {
        return false;
    }
    lower.contains("finish")
        && (lower.contains("rejects secret")
            || lower.contains("rejects derivedfromsecret")
            || lower.contains("rejects finish")
            || (lower.contains("rejects") && lower.contains("result taint")))
}

fn finish_edit_needed(text: &str) -> bool {
    !text.contains("Finished(SlotValue, Taint)") || contains_case_insensitive(text, "omits")
}
