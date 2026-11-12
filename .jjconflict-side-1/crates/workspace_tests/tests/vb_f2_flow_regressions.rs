#[path = "vb_f2_flow_regressions/support.rs"]
mod support;
use support::{Probe, TestResult};
use vb_core::{SlotValue, Taint};

const ERROR_ONLY: &str = include_str!("vb_f2_flow_regressions/error_only_nested_budget.yaml");
const EXITED_SCOPE: &str =
    include_str!("vb_f2_flow_regressions/together_error_then_new_scope_minimal.yaml");
const NESTED: &str = include_str!("vb_f2_flow_regressions/together_nested_single_result.yaml");

#[test]
fn error_only_handler_has_shared_cold_and_runtime_budget() -> TestResult {
    for quantum in [1, 64] {
        let mut probe = Probe::new(ERROR_ONLY)?;
        probe.seed(SlotValue::I64(0), Taint::Clean)?;
        let budget = probe.budget()?;
        assert_eq!(budget.max_parallel_in_flight, 4);
        let policy = vb_core::budget::BoundednessPolicy {
            absolute_max_parallel: 3,
            ..vb_core::budget::BoundednessPolicy::DEFAULT
        };
        assert_eq!(
            policy.validate(&budget),
            Err(vb_core::budget::BudgetError::ParallelExceeded {
                actual: 4,
                limit: 3
            })
        );
        let result = probe.finish(quantum)?;
        assert_eq!(result, SlotValue::I64(42));
        assert_eq!(
            probe.frame.max_parallel_in_flight(),
            budget.max_parallel_in_flight
        );
        if quantum == 1 {
            assert_eq!(probe.peak, 4);
        }
        assert!(probe.frame.executed() <= budget.max_total_steps);
        probe.assert_failure_evidence()?;
    }
    Ok(())
}

#[test]
fn failed_together_releases_only_exited_scope_before_recovery() -> TestResult {
    for quantum in [1, 64] {
        let mut probe = Probe::new(EXITED_SCOPE)?;
        probe.seed(SlotValue::I64(0), Taint::Clean)?;
        assert_eq!(probe.budget()?.max_parallel_in_flight, 1);
        let result = probe.finish(quantum)?;
        assert_eq!(probe.list(result)?, &[SlotValue::I64(42)]);
        probe.assert_failure_evidence()?;
        probe.assert_result_evidence(result, Taint::Clean)?;
    }
    Ok(())
}

#[test]
fn final_nested_together_retains_typed_list() -> TestResult {
    for quantum in [1, 64] {
        let mut probe = Probe::new(NESTED)?;
        let result = probe.finish(quantum)?;
        let nested = *probe.list(result)?.first().ok_or("missing nested result")?;
        assert_eq!(probe.list(result)?.len(), 1);
        assert_eq!(probe.list(nested)?, &[SlotValue::I64(7)]);
        probe.assert_result_evidence(result, Taint::Clean)?;
    }
    Ok(())
}

#[test]
fn recovery_inside_outer_together_retains_outer_reservation() -> TestResult {
    let source = r#"version: velvet-ballistics/v1
name: recovery_inside_outer
when: {manual: {}}
inputs: {divisor: number}
steps:
  - id: outer
    together:
      branches:
        - label: first
          steps:
            - id: inner
              on_error: {handler: recovery}
              together:
                branches:
                  - label: failing
                    steps:
                      - id: fault
                        set: {output: quotient, value: '1 / $input.divisor'}
            - id: recovery
              together:
                branches:
                  - label: recovered
                    steps:
                      - id: answer
                        set: {output: answer, value: '42'}
        - label: second
          steps:
            - id: other
              set: {output: other, value: '9'}
  - id: done
    finish: {result: '$step.outer'}
"#;
    for quantum in [1, 64] {
        let mut probe = Probe::new(source)?;
        probe.seed(SlotValue::I64(0), Taint::Clean)?;
        assert_eq!(probe.budget()?.max_parallel_in_flight, 3);
        let result = probe.finish(quantum)?;
        let values = probe.list(result)?;
        assert_eq!(values.len(), 2);
        assert_eq!(
            probe.list(*values.first().ok_or("first")?)?,
            &[SlotValue::I64(42)]
        );
        assert_eq!(values.get(1), Some(&SlotValue::I64(9)));
        if quantum == 1 {
            assert_eq!(probe.peak, 3);
        }
        probe.assert_failure_evidence()?;
        probe.assert_result_evidence(result, Taint::Clean)?;
    }
    Ok(())
}

#[path = "vb_f2_flow_regressions/results.rs"]
mod results;
