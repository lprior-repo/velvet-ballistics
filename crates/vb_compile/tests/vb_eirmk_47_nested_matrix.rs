//! vb-eirmk.4.7: nested source family combinations and boundary diagnostics.
//!
//! Registers the compact pairwise outer/inner matrix over the scoped
//! families `ForEach`, `Collect`, `Reduce`, `Repeat`, `Choose`, `Together`,
//! plus the leaf effects `Wait`, `Ask`, `Do`, plus targeted three-level
//! mixtures. Every positive cell drives the public `compile_workflow` entry
//! point through full IR validation (`vb_core::validate_compiled_workflow`)
//! to a structural terminal: a `Finish` node whose result slot is the seed
//! output. Every family also carries malformed and one-over boundary checks
//! with typed rejections, and scope crossing (capture, shadow, escape) is
//! rejected. `NestedTogether { outer, inner }` is retained for `Together`
//! within `Together` at any depth, including via `Choose`, `Repeat` and
//! `ForEach` intermediates.
//!
//! Same verdicts hold on the direct path (`parse_workflow_source` +
//! `compile_source`), which produces byte-identical IR, so downstream IPC,
//! recovery and replay consume the same workflow on every path.
//!
//! No code here calls lowering internals; every assertion exercises a public
//! API. Supported constructors only: unsupported shapes are asserted as
//! typed rejections, never counted as coverage.
#![forbid(unsafe_code)]

use vb_compile::{CompileError, compile_source, compile_workflow, parse_workflow_source};
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, SlotIdx, StepIdx, WorkflowError, validate_compiled_workflow,
};

const HEADER: &str = "version: velvet-ballistics/v1\nname: nested_matrix\nwhen: {manual: {}}\n";
const INPUTS: &str =
    "inputs:\n  a_items: {is: list, of: any, max: 4}\n  b_items: {is: list, of: any, max: 4}\n";
const SEED: &str = "  - id: seed\n    set: {output: base, value: '42'}\n  - id: delay\n    set: {output: delay, value: '5'}\n";
const DONE: &str = "  - id: done\n    finish: {result: '$seed'}\n";

const OUTERS: [&str; 6] = ["foreach", "collect", "reduce", "repeat", "choose", "together"];
const INNERS: [&str; 9] = [
    "foreach", "collect", "reduce", "repeat", "choose", "together", "wait", "ask", "do",
];

fn compile(text: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn rejects(text: &str, predicate: impl Fn(&CompileError) -> bool) -> Result<(), String> {
    let errors = match compile_workflow(text.as_bytes()) {
        Ok(_) => return Err("unexpected source acceptance".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(predicate) {
        Ok(())
    } else {
        Err(format!("wrong diagnostic: {errors:?}"))
    }
}

fn is_nested_together(error: &CompileError) -> bool {
    matches!(
        error,
        CompileError::Workflow(WorkflowError::NestedTogether { .. })
    )
}

fn kind_matches(kind: &CompiledNodeKind, family: &str) -> bool {
    match family {
        "foreach" => matches!(kind, CompiledNodeKind::ForEachStart { .. }),
        "collect" => matches!(kind, CompiledNodeKind::CollectStart { .. }),
        "reduce" => matches!(kind, CompiledNodeKind::ReduceStart { .. }),
        "repeat" => matches!(kind, CompiledNodeKind::RepeatStart { .. }),
        "choose" => matches!(kind, CompiledNodeKind::Choose { .. }),
        "together" => matches!(kind, CompiledNodeKind::TogetherStart { .. }),
        "wait" => matches!(
            kind,
            CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. }
        ),
        "ask" => matches!(kind, CompiledNodeKind::Ask { .. }),
        "do" => matches!(kind, CompiledNodeKind::Do { .. }),
        _ => false,
    }
}

fn kind_present(workflow: &CompiledWorkflow, family: &str) -> bool {
    workflow
        .to_parts()
        .nodes
        .iter()
        .any(|node| kind_matches(&node.kind, family))
}

fn assert_terminal(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    validate_compiled_workflow(&parts).map_err(|error| format!("validate: {error:?}"))?;
    let last = parts
        .nodes
        .last()
        .ok_or_else(|| "no nodes".to_owned())?;
    let CompiledNodeKind::Finish { result } = &last.kind else {
        return Err(format!("last node is not Finish: {:?}", last.kind));
    };
    let seed = parts
        .nodes
        .first()
        .and_then(|node| node.output.slot())
        .ok_or_else(|| "seed has no output slot".to_owned())?;
    if *result == seed {
        Ok(())
    } else {
        Err(format!("finish result {result:?} is not the seed slot {seed:?}"))
    }
}

fn assert_no_nop(workflow: &CompiledWorkflow) -> Result<(), String> {
    if workflow
        .to_parts()
        .nodes
        .iter()
        .any(|node| matches!(node.kind, CompiledNodeKind::Nop))
    {
        return Err("filler Nop node emitted".to_owned());
    }
    Ok(())
}

fn assert_ask_pair(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    let ask = parts
        .nodes
        .iter()
        .find(|node| matches!(node.kind, CompiledNodeKind::Ask { .. }))
        .ok_or_else(|| "missing Ask node".to_owned())?;
    let answer = ask
        .output
        .slot()
        .ok_or_else(|| "ask has no answer slot".to_owned())?;
    let resumed_id = ask.id.get().checked_add(1).ok_or("ask id overflow")?;
    let resumed = workflow
        .node(StepIdx::new(resumed_id))
        .ok_or_else(|| "missing node after Ask".to_owned())?;
    match &resumed.kind {
        CompiledNodeKind::AskResume { answer: got } if *got == answer => Ok(()),
        other => Err(format!("expected AskResume with answer, got {other:?}")),
    }
}

fn inner_yaml(kind: &str, indent: &str) -> String {
    let leaf = format!(
        "{indent}      - id: leaf\n{indent}        set: {{output: picked, value: '1'}}\n"
    );
    match kind {
        "foreach" => format!(
            "{indent}- id: inner\n{indent}  for_each:\n{indent}    input: '$input.b_items'\n{indent}    variable: sub\n{indent}    steps:\n{leaf}"
        ),
        "collect" => format!(
            "{indent}- id: inner\n{indent}  collect:\n{indent}    variable: batch\n{indent}    source: '$input.b_items'\n{indent}    pages: 1\n{indent}    items: 1\n{indent}    steps:\n{leaf}"
        ),
        "reduce" => format!(
            "{indent}- id: inner\n{indent}  reduce:\n{indent}    variable: total\n{indent}    input: '$input.b_items'\n{indent}    initial: '0'\n{indent}    steps:\n{leaf}"
        ),
        "repeat" => format!(
            "{indent}- id: inner\n{indent}  repeat:\n{indent}    max_attempts: 2\n{indent}    steps:\n{leaf}"
        ),
        "choose" => format!(
            "{indent}- id: inner\n{indent}  choose:\n{indent}    branches:\n{indent}      - when: 'true'\n{indent}        steps:\n{indent}          - id: leaf\n{indent}            set: {{output: picked, value: '1'}}\n{indent}    otherwise: after_inner\n{indent}- id: after_inner\n{indent}  set: {{output: after_val, value: '2'}}\n"
        ),
        "together" => format!(
            "{indent}- id: inner\n{indent}  together:\n{indent}    branches:\n{indent}      - label: a\n{indent}        steps:\n{indent}          - id: leaf\n{indent}            set: {{output: picked, value: '1'}}\n"
        ),
        "wait" => format!("{indent}- id: inner\n{indent}  wait: {{timeout: '$delay'}}\n"),
        "ask" => format!("{indent}- id: inner\n{indent}  ask: {{prompt: '$base'}}\n"),
        "do" => format!("{indent}- id: inner\n{indent}  do: {{action: '0', input: '$base'}}\n"),
        _ => unreachable!(),
    }
}

fn outer_yaml(outer: &str, body: &str) -> String {
    match outer {
        "foreach" => format!(
            "  - id: outer\n    for_each:\n      input: '$input.a_items'\n      variable: entry\n      steps:\n{body}"
        ),
        "collect" => format!(
            "  - id: outer\n    collect:\n      variable: obatch\n      source: '$input.a_items'\n      pages: 1\n      items: 1\n      steps:\n{body}"
        ),
        "reduce" => format!(
            "  - id: outer\n    reduce:\n      variable: ototal\n      input: '$input.a_items'\n      initial: '0'\n      steps:\n{body}"
        ),
        "repeat" => format!(
            "  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n{body}"
        ),
        "choose" => format!(
            "  - id: outer\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n{body}      otherwise: after_outer\n  - id: after_outer\n    set: {{output: after_outer_v, value: '3'}}\n"
        ),
        "together" => format!(
            "  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n{body}"
        ),
        _ => unreachable!(),
    }
}

fn cell_source(outer: &str, inner: &str) -> String {
    let indent = if outer == "choose" || outer == "together" {
        "            "
    } else {
        "        "
    };
    let body = inner_yaml(inner, indent);
    // A `reduce` body must end with a `set`/`do` that preserves the
    // accumulator; the trailing update is the legal accumulator idiom.
    let body = if outer == "reduce" {
        format!(
            "{body}{indent}- id: update\n{indent}  set: {{output: local_value, value: '$ototal'}}\n"
        )
    } else {
        body
    };
    let wrapped = outer_yaml(outer, &body);
    format!("{HEADER}{INPUTS}steps:\n{SEED}{wrapped}{DONE}")
}

fn assert_ownership(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    for name in OUTERS.iter().chain(["wait", "ask", "do"].iter()) {
        let mut slots: Vec<SlotIdx> = Vec::new();
        for node in &parts.nodes {
            if kind_matches(&node.kind, name) {
                if let Some(slot) = node.output.slot() {
                    slots.push(slot);
                }
            }
        }
        for (index, slot) in slots.iter().enumerate() {
            if slots.iter().take(index).any(|earlier| earlier == slot) {
                return Err(format!("duplicate {name} output slot {slot:?}"));
            }
        }
    }
    Ok(())
}

fn assert_bounds(workflow: &CompiledWorkflow, family: &str, outer: &str, inner: &str) -> Result<(), String> {
    if family != outer && family != inner {
        return Ok(());
    }
    for node in &workflow.to_parts().nodes {
        match (&node.kind, family) {
            (CompiledNodeKind::ForEachStart { limit, .. }, "foreach") => {
                if limit.get() != 4 {
                    return Err(format!("foreach limit {} != 4", limit.get()));
                }
            }
            (
                CompiledNodeKind::CollectStart {
                    limit, page_size, ..
                },
                "collect",
            ) => {
                if limit.get() != 1 || *page_size != 1 {
                    return Err(format!(
                        "collect limit/page {} != 1/1",
                        limit.get()
                    ));
                }
            }
            (CompiledNodeKind::RepeatStart { max_attempts, .. }, "repeat") => {
                if max_attempts.get() != 2 {
                    return Err(format!("repeat attempts {} != 2", max_attempts.get()));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn assert_wait_node(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    let present = parts.nodes.iter().any(|node| {
        matches!(
            node.kind,
            CompiledNodeKind::WaitUntil { .. } | CompiledNodeKind::WaitEvent { .. }
        )
    });
    if present {
        Ok(())
    } else {
        Err("missing WaitUntil/WaitEvent node".to_owned())
    }
}

#[test]
fn matrix_positive_all_legal_pairs() -> Result<(), String> {
    for outer in OUTERS {
        for inner in INNERS {
            if outer == "together" && inner == "together" {
                continue;
            }
            let source = cell_source(outer, inner);
            let workflow = compile(&source)
                .map_err(|error| format!("{outer} x {inner}: {error}"))?;
            assert_terminal(&workflow)
                .map_err(|error| format!("{outer} x {inner}: {error}"))?;
            assert_no_nop(&workflow)
                .map_err(|error| format!("{outer} x {inner}: {error}"))?;
            assert_ownership(&workflow)
                .map_err(|error| format!("{outer} x {inner}: {error}"))?;
            for family in ["foreach", "collect", "repeat"] {
                assert_bounds(&workflow, family, outer, inner)
                    .map_err(|error| format!("{outer} x {inner}: {error}"))?;
            }
            if !kind_present(&workflow, outer) {
                return Err(format!("{outer} x {inner}: missing outer kind"));
            }
            if !kind_present(&workflow, inner) {
                return Err(format!("{outer} x {inner}: missing inner kind"));
            }
            // Deterministic IR: recompiling must yield identical parts.
            let again = compile(&source)
                .map_err(|error| format!("{outer} x {inner} recompile: {error}"))?;
            if workflow.to_parts().nodes != again.to_parts().nodes
                || workflow.to_parts().slot_count != again.to_parts().slot_count
            {
                return Err(format!("{outer} x {inner}: non-deterministic IR"));
            }
            if inner == "ask" {
                assert_ask_pair(&workflow)
                    .map_err(|error| format!("{outer} x ask: {error}"))?;
            }
            if inner == "wait" {
                assert_wait_node(&workflow)
                    .map_err(|error| format!("{outer} x wait: {error}"))?;
            }
            if inner == "do" && !kind_present(&workflow, "do") {
                return Err(format!("{outer} x do: missing Do kind"));
            }
        }
    }
    Ok(())
}

#[test]
fn matrix_exact_counts_representative_cells() -> Result<(), String> {
    // Pinned node counts for the diagonal and the leaf families. Any
    // layout change must update these counts deliberately.
    let cases = [
        ("foreach", "foreach", 16),
        ("collect", "collect", 10),
        ("reduce", "reduce", 14),
        ("repeat", "repeat", 12),
        ("choose", "choose", 8),
        ("foreach", "wait", 10),
        ("repeat", "ask", 9),
        ("choose", "do", 6),
        ("together", "do", 8),
    ];
    for (outer, inner, expected) in cases {
        let workflow = compile(&cell_source(outer, inner))?;
        if workflow.to_parts().nodes.len() != expected {
            return Err(format!(
                "{outer} x {inner}: expected {expected} nodes, got {}",
                workflow.to_parts().nodes.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn matrix_three_level_mixtures_positive() -> Result<(), String> {
    let mixtures = [
        // ForEach > Repeat > Collect
        format!(
            "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    for_each:\n      input: '$input.a_items'\n      variable: entry\n      steps:\n        - id: mid\n          repeat:\n            max_attempts: 2\n            steps:\n              - id: inner\n                collect:\n                  variable: batch\n                  source: '$input.b_items'\n                  pages: 1\n                  items: 1\n                  steps:\n                    - id: leaf\n                      set: {{output: picked, value: '$batch'}}\n{DONE}"
        ),
        // Repeat > Choose > Together
        format!(
            "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: mid\n          choose:\n            branches:\n              - when: 'true'\n                steps:\n                  - id: inner\n                    together:\n                      branches:\n                        - label: a\n                          steps:\n                            - id: leaf\n                              set: {{output: picked, value: '1'}}\n            otherwise: after_mid\n        - id: after_mid\n          set: {{output: after_v, value: '2'}}\n{DONE}"
        ),
        // Collect > ForEach > Reduce
        format!(
            "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    collect:\n      variable: obatch\n      source: '$input.a_items'\n      pages: 1\n      items: 1\n      steps:\n        - id: mid\n          for_each:\n            input: '$input.b_items'\n            variable: sub\n            steps:\n              - id: inner\n                reduce:\n                  variable: total\n                  input: '8'\n                  initial: '0'\n                  steps:\n                    - id: leaf\n                      set: {{output: picked, value: '$total'}}\n{DONE}"
        ),
        // Together > Choose > Repeat
        format!(
            "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: mid\n              choose:\n                branches:\n                  - when: 'true'\n                    steps:\n                      - id: inner\n                        repeat:\n                          max_attempts: 2\n                          steps:\n                            - id: leaf\n                              set: {{output: picked, value: '1'}}\n                otherwise: after_mid\n            - id: after_mid\n              set: {{output: after_v, value: '2'}}\n{DONE}"
        ),
        // Choose > ForEach > Together
        format!(
            "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: mid\n              for_each:\n                input: '$input.b_items'\n                variable: sub\n                steps:\n                  - id: inner\n                    together:\n                      branches:\n                        - label: a\n                          steps:\n                            - id: leaf\n                              set: {{output: picked, value: '$sub'}}\n            - id: after_mid\n              set: {{output: after_v, value: '2'}}\n      otherwise: after_outer\n  - id: after_outer\n    set: {{output: after_outer_v, value: '3'}}\n{DONE}"
        ),
    ];
    for (index, source) in mixtures.iter().enumerate() {
        let workflow =
            compile(source).map_err(|error| format!("mixture {index}: {error}"))?;
        assert_terminal(&workflow).map_err(|error| format!("mixture {index}: {error}"))?;
        assert_no_nop(&workflow).map_err(|error| format!("mixture {index}: {error}"))?;
        assert_ownership(&workflow)
            .map_err(|error| format!("mixture {index}: {error}"))?;
    }
    Ok(())
}

fn together_inner(indent: &str) -> String {
    format!(
        "{indent}- id: inner\n{indent}  together:\n{indent}    branches:\n{indent}      - label: x\n{indent}        steps:\n{indent}          - id: leaf\n{indent}            set: {{output: picked, value: '1'}}\n"
    )
}

#[test]
fn nested_together_rejected_at_any_depth() -> Result<(), String> {
    // Direct: Together > Together
    let direct = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n{}  - id: done\n    finish: {{result: '$seed'}}\n",
        together_inner("            ")
    );
    rejects(&direct, is_nested_together)?;
    // Via Choose: Together > Choose > Together
    let via_choose = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: mid\n              choose:\n                branches:\n                  - when: 'true'\n                    steps:\n{}                otherwise: after_mid\n            - id: after_mid\n              set: {{output: after_v, value: '2'}}\n  - id: done\n    finish: {{result: '$seed'}}\n",
        together_inner("                      ")
    );
    rejects(&via_choose, is_nested_together)?;
    // Via Repeat: Together > Repeat > Together
    let via_repeat = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: mid\n              repeat:\n                max_attempts: 2\n                steps:\n{}  - id: done\n    finish: {{result: '$seed'}}\n",
        together_inner("                  ")
    );
    rejects(&via_repeat, is_nested_together)?;
    // Via ForEach: Together > ForEach > Together
    let via_foreach = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: mid\n              for_each:\n                input: '$input.b_items'\n                variable: sub\n                steps:\n{}  - id: done\n    finish: {{result: '$seed'}}\n",
        together_inner("                  ")
    );
    rejects(&via_foreach, is_nested_together)?;
    // Via Collect: Together > Collect > Together
    let via_collect = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: mid\n              collect:\n                variable: batch\n                source: '$input.b_items'\n                pages: 1\n                items: 1\n                steps:\n{}  - id: done\n    finish: {{result: '$seed'}}\n",
        together_inner("                  ")
    );
    rejects(&via_collect, is_nested_together)?;
    // And the plain matrix cell Together x Together is the same rejection.
    let cell = cell_source("together", "together");
    rejects(&cell, is_nested_together)?;
    Ok(())
}

fn nested_in_repeat(inner_steps: &str) -> String {
    format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n{inner_steps}{DONE}"
    )
}

#[test]
fn per_family_malformed_and_one_over() -> Result<(), String> {
    // ForEach: reserved variable / at_once zero (one below one).
    let reserved = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: f\n    for_each:\n      input: '$input.a_items'\n      variable: step\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}"
    );
    rejects(&reserved, |error| {
        matches!(error, CompileError::InvalidName { field: "for_each.variable", .. })
    })?;
    let nested_reserved = nested_in_repeat(
        "        - id: f\n          for_each:\n            input: '$input.b_items'\n            variable: step\n            steps:\n              - id: leaf\n                set: {output: x, value: '1'}\n",
    );
    rejects(&nested_reserved, |error| {
        matches!(error, CompileError::InvalidName { field: "for_each.variable", .. })
    })?;
    let zero = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: f\n    for_each:\n      input: '$input.a_items'\n      variable: e\n      at_once: 0\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}"
    );
    rejects(&zero, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "for_each.at_once", .. }
        )
    })?;
    let one = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: f\n    for_each:\n      input: '$input.a_items'\n      variable: e\n      at_once: 1\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}"
    );
    let workflow = compile(&one)?;
    if !kind_present(&workflow, "foreach") {
        return Err("at_once 1 not lowered".to_owned());
    }

    // Collect: both bounds missing / pages or items zero.
    let missing = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: c\n    collect:\n      variable: batch\n      source: '$input.a_items'\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}"
    );
    rejects(&missing, |error| {
        matches!(error, CompileError::StepFieldShape { field: "collect", .. })
    })?;
    for (pages, items) in [(0, 1), (1, 0)] {
        let zero = format!("{HEADER}{INPUTS}steps:\n{SEED}  - id: c\n    collect:\n      variable: batch\n      source: '$input.a_items'\n      pages: {pages}\n      items: {items}\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}");
        // One bound at zero is rejected even when the other is positive.
        rejects(&zero, |error| {
            matches!(error, CompileError::StepFieldShape { field: "collect", .. })
        })?;
    }
    let at_one = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: c\n    collect:\n      variable: batch\n      source: '$input.a_items'\n      pages: 1\n      items: 1\n      steps:\n        - id: leaf\n          set: {{output: x, value: '$batch'}}\n{DONE}"
    );
    if !kind_present(&compile(&at_one)?, "collect") {
        return Err("collect pages/items 1 not lowered".to_owned());
    }

    // Reduce: empty body / accumulator type one-over (bool vs i64).
    let empty = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: r\n    reduce:\n      variable: total\n      input: '8'\n      initial: '0'\n      steps: []\n{DONE}"
    );
    rejects(&empty, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "reduce.steps", .. }
        )
    })?;
    let mismatch = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: r\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps:\n        - id: leaf\n          set: {{output: x, value: 'true'}}\n{DONE}"
    );
    rejects(&mismatch, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "reduce.steps", .. }
        )
    })?;
    let matching = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: r\n    reduce:\n      variable: total\n      input: '8'\n      initial: '10'\n      steps:\n        - id: leaf\n          set: {{output: x, value: '$total'}}\n{DONE}"
    );
    if !kind_present(&compile(&matching)?, "reduce") {
        return Err("reduce i64/i64 not lowered".to_owned());
    }

    // Repeat: empty body / max_attempts zero.
    let empty = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: r\n    repeat:\n      max_attempts: 2\n      steps: []\n{DONE}"
    );
    rejects(&empty, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "steps", .. }
        )
    })?;
    let zero = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: r\n    repeat:\n      max_attempts: 0\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}"
    );
    rejects(&zero, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "repeat.max_attempts", .. }
        )
    })?;
    let nested_zero = nested_in_repeat(
        "        - id: inner\n          repeat:\n            max_attempts: 0\n            steps:\n              - id: leaf\n                set: {output: x, value: '1'}\n",
    );
    rejects(&nested_zero, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "repeat.max_attempts", .. }
        )
    })?;
    let one_attempt = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: r\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: leaf\n          set: {{output: x, value: '1'}}\n{DONE}"
    );
    if !kind_present(&compile(&one_attempt)?, "repeat") {
        return Err("repeat max_attempts 1 not lowered".to_owned());
    }

    // Choose: empty branches / non-boolean condition.
    let empty = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: c\n    choose:\n      branches: []\n{DONE}"
    );
    rejects(&empty, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "choose.branches", .. }
        )
    })?;
    let nonbool = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: c\n    choose:\n      branches:\n        - when: '$base'\n          steps:\n            - id: leaf\n              set: {{output: x, value: '1'}}\n      otherwise: done\n{DONE}"
    );
    rejects(&nonbool, |error| {
        matches!(
            error,
            CompileError::TypeMismatch { field: "choose.branches[].when", .. }
        )
    })?;
    let boolean = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: c\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: leaf\n              set: {{output: x, value: '1'}}\n      otherwise: done\n{DONE}"
    );
    if !kind_present(&compile(&boolean)?, "choose") {
        return Err("choose boolean condition not lowered".to_owned());
    }

    // Together: empty branches at top and nested.
    let empty = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: t\n    together:\n      branches: []\n{DONE}"
    );
    rejects(&empty, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "together.branches", .. }
        )
    })?;
    // A nested `collect` with neither bound set is the same typed rejection.
    let nested_missing = nested_in_repeat(
        "        - id: inner\n          collect:\n            variable: batch\n            source: '$input.b_items'\n            steps:\n              - id: leaf\n                set: {output: x, value: '1'}\n",
    );
    rejects(&nested_missing, |error| {
        matches!(error, CompileError::StepFieldShape { field: "collect", .. })
    })?;

    // Wait: neither field (malformed) / one field (one-over in).
    let empty = format!("{HEADER}{INPUTS}steps:\n{SEED}  - id: w\n    wait: {{}}\n{DONE}");
    rejects(&empty, |error| {
        matches!(error, CompileError::StepFieldShape { field: "wait", .. })
    })?;
    let timeout_only = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: w\n    wait: {{timeout: '$delay'}}\n{DONE}"
    );
    assert_wait_node(&compile(&timeout_only)?)?;
    let nested_empty = nested_in_repeat("        - id: inner\n          wait: {}\n");
    rejects(&nested_empty, |error| {
        matches!(error, CompileError::StepFieldShape { field: "wait", .. })
    })?;

    // Ask: empty prompt (malformed) / suspension pair (one-over in).
    let empty = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: q\n    ask: {{prompt: ''}}\n{DONE}"
    );
    rejects(&empty, |error| {
        matches!(error, CompileError::CanonicalYaml { .. })
    })?;
    let ok = format!("{HEADER}{INPUTS}steps:\n{SEED}  - id: q\n    ask: {{prompt: '$base'}}\n{DONE}");
    assert_ask_pair(&compile(&ok)?)?;
    let bad_timeout = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: q\n    ask: {{prompt: '$base', timeout: '$nope'}}\n{DONE}"
    );
    rejects(&bad_timeout, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "nope"
        )
    })?;

    // Do: missing action / missing input (malformed), complete (in).
    let missing_action = format!("{HEADER}{INPUTS}steps:\n{SEED}  - id: d\n    do: {{input: '$base'}}\n{DONE}");
    rejects(&missing_action, |error| {
        matches!(error, CompileError::CanonicalYaml { .. })
    })?;
    let missing_input = format!("{HEADER}{INPUTS}steps:\n{SEED}  - id: d\n    do: {{action: '0'}}\n{DONE}");
    rejects(&missing_input, |error| {
        matches!(error, CompileError::CanonicalYaml { .. })
    })?;
    let complete = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: d\n    do: {{action: '0', input: '$base'}}\n{DONE}"
    );
    if !kind_present(&compile(&complete)?, "do") {
        return Err("complete do not lowered".to_owned());
    }
    Ok(())
}

#[test]
fn scope_crossing_rejected() -> Result<(), String> {
    // Inner loop variable escapes to the enclosing scope.
    let escape = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    for_each:\n      input: '$input.a_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.b_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {{output: picked, value: '$sub'}}\n  - id: after\n    set: {{output: grabbed, value: '$sub'}}\n{DONE}"
    );
    rejects(&escape, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "sub"
        )
    })?;
    // Deep output captures the outer loop variable.
    let capture = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    for_each:\n      input: '$input.a_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.b_items'\n            variable: sub\n            steps:\n              - id: leaf\n                set: {{output: entry, value: '1'}}\n{DONE}"
    );
    rejects(&capture, |error| {
        matches!(error, CompileError::DuplicateOutputName { .. })
    })?;
    // Inner variable shadows the outer variable.
    let shadow = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    for_each:\n      input: '$input.a_items'\n      variable: entry\n      steps:\n        - id: inner\n          for_each:\n            input: '$input.b_items'\n            variable: entry\n            steps:\n              - id: leaf\n                set: {{output: picked, value: '$entry'}}\n{DONE}"
    );
    rejects(&shadow, |error| {
        matches!(error, CompileError::DuplicateOutputName { .. })
    })?;
    // Collect variable escapes past the loop.
    let collect_escape = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    collect:\n      variable: obatch\n      source: '$input.a_items'\n      pages: 1\n      items: 1\n      steps:\n        - id: leaf\n          set: {{output: x, value: '$obatch'}}\n  - id: after\n    set: {{output: grabbed, value: '$obatch'}}\n{DONE}"
    );
    rejects(&collect_escape, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "obatch"
        )
    })?;
    // Choose branch output escapes the branch.
    let choose_escape = nested_in_repeat(
        "        - id: inner\n          choose:\n            branches:\n              - when: 'true'\n                steps:\n                  - id: leaf\n                    set: {output: branch_out, value: '1'}\n            otherwise: after_inner\n        - id: after_inner\n          set: {output: after_v, value: '2'}\n        - id: leak\n          set: {output: grabbed, value: '$branch_out'}\n",
    );
    rejects(&choose_escape, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "branch_out"
        )
    })?;
    // Together branch output escapes the branch.
    let together_escape = nested_in_repeat(
        "        - id: inner\n          together:\n            branches:\n              - label: a\n                steps:\n                  - id: leaf\n                    set: {output: branch_out, value: '1'}\n        - id: leak\n          set: {output: grabbed, value: '$branch_out'}\n",
    );
    rejects(&together_escape, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "branch_out"
        )
    })?;
    // Ask answer escapes the enclosing repeat body.
    let ask_escape = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: q\n          ask: {{prompt: '$base'}}\n  - id: leak\n    set: {{output: grabbed, value: '$q'}}\n{DONE}"
    );
    rejects(&ask_escape, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "q"
        )
    })?;
    // Forward reference to a nested step is rejected.
    let forward = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: q\n    ask: {{prompt: '$later'}}\n  - id: later\n    set: {{output: saved, value: '1'}}\n{DONE}"
    );
    rejects(&forward, |error| {
        matches!(
            error,
            CompileError::UnknownReferenceName { name, .. } if name.as_ref() == "later"
        )
    })?;
    Ok(())
}

#[test]
fn reduce_bare_container_is_typed_rejection() -> Result<(), String> {
    // A bare ForEach/Collect/Together as the sole reduce body does not
    // produce an accumulator value; the typed rejection names the shape.
    for container in ["foreach", "collect", "together"] {
        let indent = "        ";
        let body = inner_yaml(container, indent);
        let source = format!(
            "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    reduce:\n      variable: ototal\n      input: '$input.a_items'\n      initial: '0'\n      steps:\n{body}{DONE}"
        );
        rejects(&source, |error| {
            matches!(
                error,
                CompileError::StepFieldShape { field: "reduce.steps", .. }
            )
        })
        .map_err(|error| format!("reduce x {container}: {error}"))?;
    }
    // A bare Wait as the sole body has no set/do step at all.
    let bare_wait = format!(
        "{HEADER}{INPUTS}steps:\n{SEED}  - id: outer\n    reduce:\n      variable: ototal\n      input: '$input.a_items'\n      initial: '0'\n      steps:\n        - id: inner\n          wait: {{timeout: '$delay'}}\n{DONE}"
    );
    rejects(&bare_wait, |error| {
        matches!(
            error,
            CompileError::StepFieldShape { field: "reduce.steps", .. }
        )
    })?;
    Ok(())
}

#[test]
fn source_direct_path_parity() -> Result<(), String> {
    // Every legal matrix cell compiles identically on both paths.
    for outer in OUTERS {
        for inner in INNERS {
            if outer == "together" && inner == "together" {
                continue;
            }
            let source = cell_source(outer, inner);
            let via_workflow = compile(&source)
                .map_err(|error| format!("{outer} x {inner} source: {error}"))?;
            let parsed = parse_workflow_source(&source)
                .map_err(|error| format!("{outer} x {inner} parse: {error:?}"))?;
            let via_source = compile_source(&parsed)
                .map_err(|error| format!("{outer} x {inner} direct: {error:?}"))?;
            if via_workflow.to_parts().nodes != via_source.to_parts().nodes
                || via_workflow.to_parts().slot_count != via_source.to_parts().slot_count
            {
                return Err(format!(
                    "{outer} x {inner}: source/direct IR mismatch"
                ));
            }
            assert_terminal(&via_source)
                .map_err(|error| format!("{outer} x {inner} direct: {error}"))?;
        }
    }
    // The Together x Together rejection is identical on both paths.
    let cell = cell_source("together", "together");
    let errors = match compile_workflow(cell.as_bytes()) {
        Ok(_) => return Err("source path accepted Together x Together".to_owned()),
        Err(errors) => errors,
    };
    if !errors.0.iter().any(is_nested_together) {
        return Err(format!("source path wrong diagnostic: {errors:?}"));
    }
    let parsed = parse_workflow_source(&cell)
        .map_err(|error| format!("parse: {error:?}"))?;
    match compile_source(&parsed) {
        Ok(_) => Err("direct path accepted Together x Together".to_owned()),
        Err(errors)
            if errors.0.iter().any(is_nested_together) =>
        {
            Ok(())
        }
        Err(errors) => Err(format!("direct path wrong diagnostic: {errors:?}")),
    }
}

#[test]
fn leaf_only_terminal_parity() -> Result<(), String> {
    // Sanity: each leaf alone reaches the same terminal contract.
    for (body, label) in [
        ("  - id: leaf\n    wait: {timeout: '$delay'}\n", "wait"),
        ("  - id: leaf\n    ask: {prompt: '$base'}\n", "ask"),
        ("  - id: leaf\n    do: {action: '0', input: '$base'}\n", "do"),
    ] {
        let source = format!("{HEADER}{INPUTS}steps:\n{SEED}{body}{DONE}");
        let workflow = compile(&source)?;
        assert_terminal(&workflow)?;
        if !kind_present(&workflow, label) {
            return Err(format!("missing {label} node"));
        }
    }
    Ok(())
}







