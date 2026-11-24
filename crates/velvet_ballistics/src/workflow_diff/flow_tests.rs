//! Control-flow shaped fixtures: branching, loops, error handlers, concurrency.

use super::tests::{TestResult, diff_texts, messages};

const HEADER: &str = "version: velvet-ballistics/v1\nname: w\nwhen: {manual: {}}\n";

fn for_each(max: u32, at_once: Option<u32>) -> String {
    let at_once = at_once.map_or(String::new(), |n| format!("      at_once: {n}\n"));
    format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: {max}}}\nsteps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: each\n    for_each:\n      input: '$input.items'\n      variable: entry\n{at_once}      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn choose(second_branch_step: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: flag\n    set: {{output: go, value: 'true'}}\n  - id: pick\n    choose:\n      branches:\n        - when: '$go'\n          steps:\n            - id: a\n              set: {{output: xa, value: '1'}}\n        - when: 'true'\n          steps:\n            - id: {second_branch_step}\n              set: {{output: yb, value: '2'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn repeat(max_attempts: u32) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: again\n    repeat:\n      max_attempts: {max_attempts}\n      steps:\n        - id: inner\n          set: {{output: tick, value: '2'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn handler(target: &str) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: base, value: '1'}}\n  - id: act\n    do: {{action: '1', input: '0'}}\n    on_error: {{handler: {target}}}\n  - id: fix1\n    set: {{output: f1, value: '1'}}\n  - id: fix2\n    set: {{output: f2, value: '2'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

#[test]
fn branch_target_swap_reports_exactly_the_moved_choose_edges() -> TestResult {
    let report = diff_texts(&choose("b"), &choose("c"))?;
    assert_eq!(
        messages(&report, "control_flow"),
        [
            "edge removed: b:set_const -> done:finish [next]",
            "edge removed: pick:choose -> b:set_const [goto]",
            "edge added: c:set_const -> done:finish [next]",
            "edge added: pick:choose -> c:set_const [goto]",
        ]
    );
    assert_eq!(
        messages(&report, "steps"),
        ["step removed: b:set_const", "step added: c:set_const"]
    );
    assert_eq!(report.old_steps, report.new_steps);
    Ok(())
}

#[test]
fn error_handler_retarget_reports_the_on_error_edge_only() -> TestResult {
    let report = diff_texts(&handler("fix1"), &handler("fix2"))?;
    assert_eq!(
        messages(&report, "control_flow"),
        [
            "edge removed: act:do -> fix1:set_const [on_error]",
            "edge added: act:do -> fix2:set_const [on_error]",
        ]
    );
    assert_eq!(report.changes.len(), 2, "{:?}", report.changes);
    Ok(())
}

#[test]
fn removing_an_error_handler_removes_its_edge() -> TestResult {
    let without = handler("fix1").replace("    on_error: {handler: fix1}\n", "");
    let report = diff_texts(&handler("fix1"), &without)?;
    assert_eq!(
        messages(&report, "control_flow"),
        ["edge removed: act:do -> fix1:set_const [on_error]"]
    );
    Ok(())
}

#[test]
fn loop_bound_change_is_a_semantic_change_even_when_edges_are_equal() -> TestResult {
    let report = diff_texts(&repeat(2), &repeat(3))?;
    assert!(report.has_semantic_changes());
    assert!(
        messages(&report, "control_flow").is_empty(),
        "{:?}",
        report.changes
    );
    assert_eq!(report.changes.len(), 1, "{:?}", report.changes);
    assert_eq!(report.changes.first().map(|c| c.category), Some("digest"));
    assert!(!report.content_equal);
    Ok(())
}

#[test]
fn concurrency_width_change_is_a_semantic_change() -> TestResult {
    let report = diff_texts(&for_each(4, Some(2)), &for_each(4, Some(3)))?;
    assert!(report.has_semantic_changes());
    assert!(!report.content_equal);
    Ok(())
}

#[test]
fn input_list_bound_moves_the_resource_contract_limit() -> TestResult {
    let report = diff_texts(&for_each(4, None), &for_each(9000, None))?;
    assert_eq!(
        messages(&report, "resource_contract"),
        ["limit max_collect_items changed: 1024 -> 9000"]
    );
    let reverse = diff_texts(&for_each(9000, None), &for_each(4, None))?;
    assert_eq!(
        messages(&reverse, "resource_contract"),
        ["limit max_collect_items changed: 9000 -> 1024"]
    );
    Ok(())
}
