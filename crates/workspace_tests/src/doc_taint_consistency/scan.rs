//! Pure text-to-findings policy evaluation.
//!
//! Returns findings in the exact order required by the contract:
//! node findings (line order), vocabulary findings (declared order),
//! control-flow findings (declared order), stale finish finding,
//! finish rejection contradictions (line order), missing required wording.

use crate::doc_taint_consistency::rules::*;
use crate::doc_taint_consistency::splitlines::python_splitlines;

pub(crate) fn scan(text: &str) -> Vec<String> {
    let lines = python_splitlines(text);

    let node_findings = node_findings_from_lines(&lines);
    let vocab_findings = vocabulary_findings(text);
    let cf_findings = control_flow_findings(text);
    let stale_finish = stale_finish_finding(text);
    let rejection_findings = finish_rejection_findings(&lines);
    let missing_required = !text.contains(FINISH_REQUIRED);

    let mut findings = Vec::new();
    findings.extend(node_findings);
    findings.extend(vocab_findings);
    findings.extend(cf_findings);
    if stale_finish {
        findings.push("stale finish signal missing taint".to_string());
    }
    findings.extend(rejection_findings);
    if missing_required {
        findings.push("missing finish signal taint wording".to_string());
    }

    findings
}

fn node_findings_from_lines(lines: &[String]) -> Vec<String> {
    let numbered = lines
        .iter()
        .zip(1..=lines.len())
        .map(|(line, n)| (line.as_str(), n));

    numbered
        .flat_map(|(line, line_number)| find_node_findings_for_line(line, line_number))
        .collect()
}

fn find_node_findings_for_line(line: &str, line_number: usize) -> Vec<String> {
    NODE_RULES
        .iter()
        .flat_map(move |rule| find_findings_for_rule(line, line_number, rule))
        .collect()
}

fn find_findings_for_rule<'a>(
    line: &'a str,
    line_number: usize,
    rule: &'a NodeRule,
) -> Vec<String> {
    if !line.contains(rule.name) {
        return Vec::new();
    }

    let phrase_findings = rule
        .phrases
        .iter()
        .filter(|phrase| line.contains(*phrase))
        .map(|phrase| format_finding(line_number, rule.name, phrase))
        .collect::<Vec<String>>();

    let writer_finding =
        if line.contains("write_slot") && line.contains("not write_slot_with_taint") {
            Some(format_finding(line_number, rule.name, "write_slot"))
        } else {
            None
        };

    let mut results = phrase_findings;
    if let Some(finding) = writer_finding {
        results.push(finding);
    }
    results
}

fn vocabulary_findings(text: &str) -> Vec<String> {
    VOCABULARY_CONFLICTS
        .iter()
        .filter(|vc| text.contains(vc.phrase))
        .map(|vc| vc.reason.to_string())
        .collect()
}

fn control_flow_findings(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    CONTROL_FLOW_CONFLICTS
        .iter()
        .filter(|phrase| lower.contains(*phrase))
        .map(|phrase| format!("control-flow taint conflation: {}", phrase))
        .collect()
}

fn stale_finish_finding(text: &str) -> bool {
    let target = "Finished(SlotValue)";
    text.match_indices(target)
        .any(|(start, _)| match start.checked_add(target.len()) {
            None => true,
            Some(after) => match text.get(after..) {
                None => true,
                Some(remainder) => !remainder.starts_with(','),
            },
        })
}

fn is_finish_rejection_contradiction(line: &str) -> bool {
    let lower = line.to_lowercase();
    let has_taint = lower.contains("secret") || lower.contains("derivedfromsecret");
    let has_reject = lower.contains("reject") || lower.contains("rejection");
    let allowed = lower.contains("no rejection")
        || lower.contains("does not reject")
        || lower.contains("not reject");
    lower.contains("finish") && has_taint && has_reject && !allowed
}

fn finish_rejection_findings(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| is_finish_rejection_contradiction(line))
        .map(|_| "finish rejection contradiction".to_string())
        .collect()
}

fn format_finding(line_number: usize, node: &str, phrase: &str) -> String {
    format!(
        "line {}: {} stale taint wording: {}",
        line_number, node, phrase
    )
}

#[cfg(test)]
#[path = "scan_tests.rs"]
mod tests;
