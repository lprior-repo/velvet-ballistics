use crate::{DocReconcileError, ResolvedNode, StalePhrase};

pub(super) struct Contradictions {
    pub(super) all_stale_phrases: Vec<StalePhrase>,
    pub(super) first_error: Option<DocReconcileError>,
    pub(super) count: usize,
    pub(super) has_eval_expr: bool,
    pub(super) has_build_object: bool,
    pub(super) has_build_list: bool,
}

pub(super) fn collect(text: &str) -> Contradictions {
    let mut findings = Contradictions {
        all_stale_phrases: Vec::new(),
        first_error: None,
        count: 0,
        has_eval_expr: false,
        has_build_object: false,
        has_build_list: false,
    };
    add_node_findings(text, ResolvedNode::EvalExpr, &mut findings);
    add_node_findings(text, ResolvedNode::BuildObject, &mut findings);
    add_node_findings(text, ResolvedNode::BuildList, &mut findings);
    add_finish_findings(text, &mut findings);
    findings
}

pub(super) fn scanned_nodes() -> Vec<ResolvedNode> {
    vec![
        ResolvedNode::EvalExpr,
        ResolvedNode::BuildObject,
        ResolvedNode::BuildList,
        ResolvedNode::Finish,
    ]
}

fn add_node_findings(text: &str, node: ResolvedNode, findings: &mut Contradictions) {
    text.lines()
        .filter(|line| line.contains(node_name(node)))
        .for_each(|line| {
            add_always_clean(line, node, findings);
            add_no_join(line, node, findings);
            add_write_slot(line, node, findings);
        });
}

fn add_always_clean(text: &str, node: ResolvedNode, findings: &mut Contradictions) {
    if text.contains("Always Clean") || text.contains("always Clean") {
        add_finding(node, "Always Clean", stale_always_clean(node), findings);
    }
}

fn add_no_join(text: &str, node: ResolvedNode, findings: &mut Contradictions) {
    let phrase = no_join_phrase(text, node);
    if let Some(value) = phrase {
        add_finding(node, value, stale_no_join(node), findings);
    }
}

fn add_write_slot(text: &str, node: ResolvedNode, findings: &mut Contradictions) {
    if text.contains("write_slot") && text.contains("not write_slot_with_taint") {
        add_finding(node, "write_slot", StalePhrase::WriteSlotOnly, findings);
    }
}

fn add_finish_findings(text: &str, findings: &mut Contradictions) {
    text.lines()
        .filter(|line| line.contains("Finish"))
        .for_each(|line| add_finish_line_findings(line, findings));
}

fn add_finish_line_findings(line: &str, findings: &mut Contradictions) {
    if line.contains("Finished(SlotValue)") && !line.contains("Finished(SlotValue, Taint)") {
        add_finding(
            ResolvedNode::Finish,
            "Finished(SlotValue)",
            StalePhrase::WriteSlotOnly,
            findings,
        );
    }
    if contains_finish_rejection(line) {
        add_finding(
            ResolvedNode::Finish,
            "rejects finish taint",
            StalePhrase::WriteSlotOnly,
            findings,
        );
    }
}

fn contains_finish_rejection(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let allowed = lower.contains("no rejection")
        || lower.contains("does not reject")
        || lower.contains("not reject");
    lower.contains("finish")
        && lower.contains("reject")
        && (lower.contains("secret") || lower.contains("derivedfromsecret"))
        && !allowed
}

fn add_finding(
    node: ResolvedNode,
    phrase: &str,
    stale_phrase: StalePhrase,
    findings: &mut Contradictions,
) {
    mark_node(node, findings);
    findings.count = findings.count.saturating_add(1);
    findings.all_stale_phrases.push(stale_phrase);
    if findings.first_error.is_none() {
        findings.first_error = Some(DocReconcileError::StaleCleanOnlyTaintText {
            node,
            phrase: phrase.to_owned(),
        });
    }
}

fn mark_node(node: ResolvedNode, findings: &mut Contradictions) {
    match node {
        ResolvedNode::EvalExpr => findings.has_eval_expr = true,
        ResolvedNode::BuildObject => findings.has_build_object = true,
        ResolvedNode::BuildList => findings.has_build_list = true,
        ResolvedNode::Finish => {}
    }
}

fn node_name(node: ResolvedNode) -> &'static str {
    match node {
        ResolvedNode::EvalExpr => "EvalExpr",
        ResolvedNode::BuildObject => "BuildObject",
        ResolvedNode::BuildList => "BuildList",
        ResolvedNode::Finish => "Finish",
    }
}

fn stale_always_clean(node: ResolvedNode) -> StalePhrase {
    match node {
        ResolvedNode::EvalExpr => StalePhrase::EvalExprAlwaysClean,
        ResolvedNode::BuildObject => StalePhrase::BuildObjectAlwaysClean,
        ResolvedNode::BuildList => StalePhrase::BuildListAlwaysClean,
        ResolvedNode::Finish => StalePhrase::WriteSlotOnly,
    }
}

fn stale_no_join(node: ResolvedNode) -> StalePhrase {
    match node {
        ResolvedNode::EvalExpr => StalePhrase::EvalExprNoOperandJoin,
        ResolvedNode::BuildObject => StalePhrase::BuildObjectNoFieldJoin,
        ResolvedNode::BuildList => StalePhrase::BuildListNoItemJoin,
        ResolvedNode::Finish => StalePhrase::WriteSlotOnly,
    }
}

fn no_join_phrase(text: &str, node: ResolvedNode) -> Option<&'static str> {
    if text.contains("No taint join") {
        Some("No taint join")
    } else if node == ResolvedNode::BuildObject && text.contains("no join of field taints") {
        Some("no join of field taints")
    } else if node == ResolvedNode::BuildList && text.contains("no join of item taints") {
        Some("no join of item taints")
    } else {
        None
    }
}
