use vb_core::{CompiledNodeKind as K, workflow::WorkflowIrProfile};

fn indent(source: &str, padding: &str) -> String {
    source
        .lines()
        .map(|line| format!("{padding}{line}\n"))
        .collect()
}

fn source(cap: u32, body: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: review_nested\nwhen: {{manual: {{}}}}\nvars: {{xs: [1, 2]}}\nsteps:\n  - id: seed\n    set: {{output: answer, value: '42'}}\n  - id: outer\n    for_each:\n      variable: entry\n      input: '$vars.xs'\n      at_once: {cap}\n      steps:\n{}  - id: done\n    finish: {{result: answer}}\n",
        indent(body, "        ")
    )
}

fn wrappers(leaf: &str) -> Vec<String> {
    vec![
        leaf.to_owned(),
        format!(
            "- id: nested\n  repeat:\n    max_attempts: 1\n    steps:\n{}",
            indent(leaf, "      ")
        ),
        format!(
            "- id: nested\n  for_each:\n    variable: inner\n    input: '$vars.xs'\n    at_once: 2\n    steps:\n{}",
            indent(leaf, "      ")
        ),
        format!(
            "- id: nested\n  choose:\n    branches:\n      - when: 'true'\n        steps:\n{}",
            indent(leaf, "          ")
        ),
        format!(
            "- id: nested\n  together:\n    branches:\n      - label: branch\n        steps:\n{}",
            indent(leaf, "          ")
        ),
        format!(
            "- id: nested\n  collect:\n    variable: batch\n    source: '$vars.xs'\n    pages: 2\n    items: 1\n    steps:\n{}",
            indent(leaf, "      ")
        ),
        format!(
            "- id: nested\n  reduce:\n    variable: accumulator\n    input: '$vars.xs'\n    initial: '0'\n    steps:\n{}      - id: reduced\n        set: {{output: reduced, value: '0'}}\n",
            indent(leaf, "      ")
        ),
    ]
}

#[test]
fn review_all_nested_suspend_shapes_use_serial_foreach() -> Result<(), String> {
    for leaf in [
        "- id: leaf\n  ask: {prompt: '$entry'}\n",
        "- id: leaf\n  wait: {timeout: '$answer'}\n",
        "- id: leaf\n  do: {action: '0', input: '$entry'}\n",
    ] {
        for body in wrappers(leaf) {
            for cap in [1, 2, u32::MAX] {
                let yaml = source(cap, &body);
                let plan = vb_compile::compile_workflow(yaml.as_bytes())
                    .map_err(|error| format!("{yaml}\n{error:?}"))?;
                assert_eq!(plan.ir_profile(), WorkflowIrProfile::LegacyV2, "{yaml}");
                let parts = plan.to_parts();
                assert!(
                    parts
                        .nodes
                        .iter()
                        .any(|node| matches!(node.kind, K::ForEachStart { .. }))
                );
                assert!(
                    !parts
                        .nodes
                        .iter()
                        .any(|node| matches!(node.kind, K::ForEachConcurrentStart { .. }))
                );
            }
        }
    }
    Ok(())
}

#[test]
fn review_pure_nested_foreach_remains_concurrent() -> Result<(), String> {
    let leaf = "- id: leaf\n  set: {output: result, value: '$inner'}\n";
    let body = format!(
        "- id: nested\n  for_each:\n    variable: inner\n    input: '$vars.xs'\n    at_once: 2\n    steps:\n{}",
        indent(leaf, "      ")
    );
    let plan = vb_compile::compile_workflow(source(2, &body).as_bytes())
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(plan.ir_profile(), WorkflowIrProfile::ConcurrentV3);
    assert_eq!(
        plan.to_parts()
            .nodes
            .iter()
            .filter(|node| matches!(node.kind, K::ForEachConcurrentStart { .. }))
            .count(),
        2
    );
    Ok(())
}
