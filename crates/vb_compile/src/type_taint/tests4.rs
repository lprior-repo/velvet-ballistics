}

#[test]
fn type_taint_errors_preempt_control_flow_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 3
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}

#[test]
fn type_taint_errors_preempt_backward_branch_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 0
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}

#[test]
fn type_taint_errors_preempt_self_branch_errors() -> Result<(), String> {
    let source = br#"version: velvet-ballastics/v1
name: preempt_case
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: route
    choose:
      condition: 0
      on_true: 1
      on_false: 2
  - id: done
    finish:
      result: 0
"#;

    ensure_pair(source, ensure_choose_type_mismatch)
}
