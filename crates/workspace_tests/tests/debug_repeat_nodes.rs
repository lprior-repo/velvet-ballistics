#[test]
fn debug_repeat_nodes() {
    use vb_compile::compile_workflow;
    use vb_core::CompiledNodeKind;

    let source = r#"version: velvet-ballistics/v1
name: f1_repeat_counter_probe
when:
  manual: {}
steps:
  - id: first
    set:
      output: answer
      value: "42"
  - id: bounded_repeat
    repeat:
      max_attempts: 3
      steps:
        - id: body_value
          set:
            output: body_result
            value: "12884901888"
  - id: done
    finish:
      result: answer
"#;

    let workflow = compile_workflow(source.as_bytes()).expect("compile failed");
    let parts = workflow.to_parts();

    println!("Node count: {}", parts.nodes.len());
    for (i, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::RepeatStart { body, done, .. } => {
                println!("Node {}: RepeatStart body={:?} done={:?} output={:?}", i, body, done, node.output);
            }
            CompiledNodeKind::RepeatCheck { attempt_slot, done, .. } => {
                println!("Node {}: RepeatCheck attempt={:?} done={:?} output={:?} next={:?}", i, attempt_slot, done, node.output, node.next);
            }
            CompiledNodeKind::RepeatFinish { result } => {
                println!("Node {}: RepeatFinish result={:?} output={:?}", i, result, node.output);
            }
            CompiledNodeKind::SetConst { value } => {
                let const_value = workflow.constant(*value).unwrap();
                println!("Node {}: SetConst value={:?} output={:?}", i, const_value, node.output);
            }
            other => {
                println!("Node {}: {:?}", i, other);
            }
        }
    }
}
