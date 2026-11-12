#![forbid(unsafe_code)]

use vb_compile::{CompileError, compile_workflow};
use vb_core::SlotIdx;

fn yaml(steps: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: guards\nwhen: {{ manual: {{}} }}\nsteps:\n{steps}"
    )
}

fn expect_err(yaml: &str, expected: CompileError) -> Result<(), String> {
    match compile_workflow(yaml.as_bytes()) {
        Ok(_) => Err(format!("expected {:?}, got Ok", expected)),
        Err(errors) => {
            for err in errors.0.iter() {
                if *err == expected {
                    return Ok(());
                }
            }
            Err(format!("expected {:?}, got {:?}", expected, errors.0))
        }
    }
}

#[test]
fn guard_consumer_ask_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: a\n    ask: { prompt: \"$g\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "ask.prompt",
        },
    )
}

#[test]
fn guard_consumer_wait_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: w\n    wait: { event: \"$g\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "waitevent.event",
        },
    )
}

#[test]
fn guard_consumer_ask_timeout_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: a\n    ask: { prompt: \"$seed\", timeout: \"$g\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "ask.timeout",
        },
    )
}

#[test]
fn guard_consumer_wait_timeout_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: w\n    wait: { event: \"$seed\", timeout: \"$g\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "waitevent.timeout",
        },
    )
}

#[test]
fn guard_consumer_waituntil_deadline_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: w\n    wait: { timeout: \"$g\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "waituntil.deadline",
        },
    )
}

const SOURCE: &str = r#"version: velvet-ballistics/v1
name: suspend
when:
  manual: {}
steps:
  - id: seed
    set:
      output: alias
      value: "42"
  - id: clock
    set:
      output: duration
      value: "600000"
  - id: question
    ask:
      prompt: "$alias"
      timeout: "$steps.clock"
  - id: done
    finish:
      result: "$seed""#;

fn derive_wait(src: &str) -> String {
    src.replace("ask:", "wait:").replace("prompt:", "event:")
}

#[test]
fn unknown_suspension_references() -> Result<(), String> {
    let bases = [
        (false, "$alias"),
        (false, "$steps.clock"),
        (true, "$alias"),
        (true, "$steps.clock"),
    ];

    let refs = [
        ("$missing", "output", "missing"),
        ("$question", "output", "question"),
        ("$steps.missing", "step", "missing"),
        ("$input.missing", "declaration", "$input.missing"),
        ("$vars.missing", "declaration", "$vars.missing"),
    ];

    for (use_wait, old) in bases {
        let base = if use_wait {
            derive_wait(SOURCE)
        } else {
            SOURCE.to_owned()
        };
        for (reference, kind, name) in refs {
            let src = base.replace(old, reference);
            expect_err(
                &src,
                CompileError::UnknownReferenceName {
                    kind: kind,
                    reference: Box::from(reference),
                    name: Box::from(name),
                },
            )?;
        }
    }

    Ok(())
}

#[test]
fn numeric_suspension_slot_boundaries() -> Result<(), String> {
    let cases = [
        (false, "$alias", "ask.prompt"),
        (false, "$steps.clock", "ask.timeout"),
        (true, "$alias", "wait.event"),
        (true, "$steps.clock", "wait.timeout"),
    ];
    for (wait, old, _field) in cases.iter() {
        let base = if *wait {
            derive_wait(SOURCE)
        } else {
            SOURCE.to_owned()
        };
        for value in [-1_i64, 65535_i64, 65536_i64].iter() {
            let modified = base.replace(old, &value.to_string());
            expect_err(
                &modified,
                CompileError::SlotIndexOutOfRange { value: *value },
            )?;
        }
    }
    Ok(())
}

#[path = "vb_f1_suspension_guard_reads/malformed.rs"]
mod malformed;

#[test]
fn ambiguous_seed_reference() -> Result<(), String> {
    let base = SOURCE
        .replace("output: duration", "output: seed")
        .replace("result: \"$seed\"", "result: \"$steps.seed\"");

    let variants = [
        (false, "$alias"),
        (false, "$steps.clock"),
        (true, "$alias"),
        (true, "$steps.clock"),
    ];

    for (use_wait, old) in variants {
        let src = if use_wait {
            derive_wait(&base)
        } else {
            base.clone()
        };
        let modified = src.replace(old, "$seed");
        assert!(modified != src, "transformation not effective");
        expect_err(
            &modified,
            CompileError::ExpressionLoweringUnsupported {
                feature: Box::from(
                    "ambiguous reference 'seed' found in both output names and step IDs with different slots",
                ),
            },
        )?;
    }
    Ok(())
}

#[test]
fn forward_clock_reference_rejected() -> Result<(), String> {
    let clock_block = "  - id: clock\n    set:\n      output: duration\n      value: \"600000\"\n";
    let without_clock = SOURCE.replace(clock_block, "");
    let modified = without_clock.replace("  - id: done", &format!("{}  - id: done", clock_block));
    assert!(modified != SOURCE, "transformation not effective");
    expect_err(
        &modified,
        CompileError::UnknownReferenceName {
            kind: "step",
            reference: Box::from("$steps.clock"),
            name: Box::from("clock"),
        },
    )?;
    Ok(())
}
