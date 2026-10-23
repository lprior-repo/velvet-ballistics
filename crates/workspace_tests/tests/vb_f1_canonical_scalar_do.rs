#![forbid(unsafe_code)]
//! F1 bounded slice: canonical scalar Set + Do action round-trip.
//! Source-to-runtime regression oracles for vb-8eocj.2.

use vb_core::{ActionId, RunId, SlotIdx, SlotValue, StepIdx};
use vb_f1_scalar_do_support::{
    compile_f1_workflow, F1SourceRun, assert_finished_with_value, test_runtime, action_output,
};

fn yaml_set_finish(output: &str, value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: f1_set\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: s1\n    set:\n      output: {output}\n      value: \"{value}\"\n  - id: s2\n    finish:\n      result: 0\n"
    )
}

fn yaml_set_expr_finish(value: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: f1_expr\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: s1\n    set:\n      output: x\n      value: \"10\"\n  - id: s2\n    set:\n      output: result\n      value: \"{value}\"\n  - id: s3\n    finish:\n      result: 1\n"
    )
}

fn yaml_step_ref_finish(ref_syntax: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: f1_ref\nwhen: {{ manual: {{}} }}\nsteps:\n  - id: producer\n    set:\n      output: out\n      value: \"42\"\n  - id: consumer\n    finish:\n      result: {ref_syntax}\n"
    )
}

const DO_YAML: &str = "version: velvet-ballistics/v1\nname: f1_do\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set:\n      output: input_val\n      value: \"10\"\n  - id: s2\n    do:\n      action: \"7\"\n      input: \"0\"\n  - id: s3\n    finish:\n      result: 1\n";

// Positive: I64 scalar Set -> Finish
#[test]
fn test_i64_set_unchanged_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("answer", "42"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: Bool scalar Set -> Finish
#[test]
fn test_bool_set_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("flag", "true"))?;
    assert_finished_with_value(&mut run, SlotValue::Bool(true))
}

// Positive: Null scalar Set -> Finish
#[test]
fn test_null_set_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("n", "null"))?;
    assert_finished_with_value(&mut run, SlotValue::Null)
}

// Positive: F64 scalar Set -> Finish (finite)
#[test]
fn test_f64_set_to_finish() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_finish("ratio", "3.14"))?;
    let expected = SlotValue::F64(vb_core::FiniteF64::new(3.14).map_err(|e| format!("FiniteF64: {e:?}"))?);
    assert_finished_with_value(&mut run, expected)
}

// Positive: arithmetic expression through prior output $alias
#[test]
fn test_arithmetic_through_prior_output_alias() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_set_expr_finish("$x + 2"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(12))
}

// Positive: step reference syntax $step_id
#[test]
fn test_step_reference_by_id() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_step_ref_finish("$producer.out"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: step reference syntax $step.id
#[test]
fn test_step_reference_qualified() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_step_ref_finish("$step.producer.out"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: step reference syntax $steps.id
#[test]
fn test_step_reference_steps_prefix() -> Result<(), String> {
    let mut run = F1SourceRun::new(&yaml_step_ref_finish("$steps.producer.out"))?;
    assert_finished_with_value(&mut run, SlotValue::I64(42))
}

// Positive: real Do suspension + completion to Finish
#[test]
fn test_do_action_suspension_and_completion() -> Result<(), String> {
    let workflow = compile_f1_workflow(DO_YAML)?;
    let mut runtime = test_runtime()?;
    let run = RunId::new(1001);
    runtime.submit_direct(run, workflow).map_err(|e| format!("submit failed: {e:?}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed: {e:?}"))?;

    let ticket = runtime.lookup_pending_action_ticket(run, 1)
        .ok_or("expected pending action ticket for Do step")?;
    if ticket.action != ActionId::new(7) {
        return Err(format!("action ticket action ID mismatch: {:#?}", ticket.action));
    }
    if ticket.step != StepIdx::new(1) {
        return Err(format!("action ticket step mismatch: {:#?}", ticket.step));
    }

    runtime.complete_action_with_output(ticket, action_output(SlotValue::I64(999)))
        .map_err(|e| format!("action completion failed: {e:?}"))?;
    runtime.tick_all().map_err(|e| format!("tick failed after completion: {e:?}"))?;

    let counters = runtime.counters_snapshot();
    if counters.runs_completed != 1 {
        return Err(format!("expected 1 completed run, got {}", counters.runs_completed));
    }
    Ok(())
}

// Negative: duplicate output name
#[test]
fn test_duplicate_output_name_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: dup\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    set: { output: x, value: \"2\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("duplicate") || msg.contains("Duplicate") || msg.contains("already") => Ok(()),
        Err(msg) => Err(format!("expected duplicate name error, got: {msg}")),
        Ok(_) => Err("expected compile error for duplicate output name, got success".to_owned()),
    }
}

// Negative: undefined name reference in finish
#[test]
fn test_undefined_name_reference_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: undef\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    finish: { result: \"missing\" }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("undefined") || msg.contains("Undefined") || msg.contains("unknown") || msg.contains("Unknown") => Ok(()),
        Err(msg) => Err(format!("expected undefined name error, got: {msg}")),
        Ok(_) => Err("expected compile error for undefined name, got success".to_owned()),
    }
}

// Negative: forward reference
#[test]
fn test_forward_reference_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: fwd\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"$steps.future.out\" }\n  - id: future\n    set: { output: out, value: \"1\" }\n  - id: s2\n    finish: { result: 0 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("forward") || msg.contains("Forward") || msg.contains("not yet") || msg.contains("future") => Ok(()),
        Err(msg) => Err(format!("expected forward reference error, got: {msg}")),
        Ok(_) => Err("expected compile error for forward reference, got success".to_owned()),
    }
}

// Negative: self reference
#[test]
fn test_self_reference_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: selfref\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"$steps.s1.x\" }\n  - id: s2\n    finish: { result: 0 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("self") || msg.contains("Self") || msg.contains("not yet") || msg.contains("forward") => Ok(()),
        Err(msg) => Err(format!("expected self reference error, got: {msg}")),
        Ok(_) => Err("expected compile error for self reference, got success".to_owned()),
    }
}

// Negative: ambiguous bare name vs distinct step-ID
#[test]
fn test_ambiguous_bare_name_error() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: ambig\nwhen: { manual: {} }\nsteps:\n  - id: x\n    set: { output: y, value: \"1\" }\n  - id: s2\n    set: { output: y, value: \"2\" }\n  - id: s3\n    finish: { result: \"y\" }\n";
    match compile_f1_workflow(yaml) {
        Err(_) => Ok(()),
        Ok(_) => Err("expected compile error for ambiguous/duplicate reference, got success".to_owned()),
    }
}

// Negative: invalid numeric action ID (negative)
#[test]
fn test_invalid_action_id_negative() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: inv\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    do: { action: \"-1\", input: \"0\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("action") || msg.contains("Action") || msg.contains("id") || msg.contains("Id") || msg.contains("shape") || msg.contains("Shape") => Ok(()),
        Err(msg) => Err(format!("expected action ID error, got: {msg}")),
        Ok(_) => Err("expected compile error for invalid action ID, got success".to_owned()),
    }
}

// Negative: invalid numeric action ID (too large)
#[test]
fn test_invalid_action_id_too_large() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: invlarge\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    do: { action: \"70000\", input: \"0\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("action") || msg.contains("Action") || msg.contains("id") || msg.contains("Id") || msg.contains("shape") || msg.contains("Shape") || msg.contains("limit") || msg.contains("Limit") => Ok(()),
        Err(msg) => Err(format!("expected action ID error, got: {msg}")),
        Ok(_) => Err("expected compile error for large action ID, got success".to_owned()),
    }
}

// Negative: invalid input slot (negative)
#[test]
fn test_invalid_input_slot_negative() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: invslot\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    do: { action: \"1\", input: \"-1\" }\n  - id: s3\n    finish: { result: 0 }\n";
    match compile_f1_workflow(yaml) {
        Err(msg) if msg.contains("input") || msg.contains("Input") || msg.contains("slot") || msg.contains("Slot") || msg.contains("shape") || msg.contains("Shape") => Ok(()),
        Err(msg) => Err(format!("expected input slot error, got: {msg}")),
        Ok(_) => Err("expected compile error for invalid input slot, got success".to_owned()),
    }
}

// Negative: finish with expression (not in bounded F1 slice)
#[test]
fn test_finish_expression_not_supported() -> Result<(), String> {
    let yaml = "version: velvet-ballistics/v1\nname: finishexpr\nwhen: { manual: {} }\nsteps:\n  - id: s1\n    set: { output: x, value: \"1\" }\n  - id: s2\n    finish: { result: \"x + 1\" }\n";
    match compile_f1_workflow(yaml) {
        Err(_) => Ok(()),
        Ok(_) => Err("expected compile error for finish expression, got success".to_owned()),
    }
}
