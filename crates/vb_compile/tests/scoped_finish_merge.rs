use vb_compile::compile_workflow;
use vb_core::CompiledNodeKind as K;

const HEADER: &str = "version: velvet-ballistics/v1\nname: scoped_finish\nwhen: {manual: {}}\n";

#[test]
fn nested_finish_preserves_bare_and_dollar_outer_aliases() -> Result<(), String> {
    for name in ["answer", "$answer"] {
        let source = format!(
            "{HEADER}steps:\n  - id: seed\n    set: {{output: answer, value: '42'}}\n  - id: loop\n    repeat:\n      max_attempts: 1\n      steps:\n        - id: early\n          if: 'true'\n          finish: {{result: '{name}'}}\n  - id: done\n    finish: {{result: answer}}\n"
        );
        let workflow =
            compile_workflow(source.as_bytes()).map_err(|error| format!("{name}: {error:?}"))?;
        let parts = workflow.to_parts();
        let results: Vec<_> = parts
            .nodes
            .iter()
            .filter_map(|node| match node.kind {
                K::Finish { result } => Some(result),
                _ => None,
            })
            .collect();
        assert_eq!(results.len(), 2);
        assert_eq!(results.first(), results.last(), "alias {name}");
    }
    Ok(())
}

#[test]
fn nested_finish_preserves_bare_and_dollar_loop_variables() -> Result<(), String> {
    for name in ["entry", "$entry"] {
        let source = format!(
            "{HEADER}vars:\n  items: [42]\nsteps:\n  - id: seed\n    set: {{output: answer, value: '42'}}\n  - id: each\n    for_each:\n      input: '$vars.items'\n      variable: entry\n      steps:\n        - id: early\n          if: 'true'\n          finish: {{result: '{name}'}}\n  - id: done\n    finish: {{result: answer}}\n"
        );
        let workflow =
            compile_workflow(source.as_bytes()).map_err(|error| format!("{name}: {error:?}"))?;
        let parts = workflow.to_parts();
        let item = parts
            .nodes
            .iter()
            .find_map(|node| match node.kind {
                K::ForEachStart { item_slot, .. } => Some(item_slot),
                _ => None,
            })
            .ok_or("missing loop item")?;
        assert!(
            parts
                .nodes
                .iter()
                .any(|node| matches!(node.kind, K::Finish { result } if result == item))
        );
    }
    Ok(())
}
