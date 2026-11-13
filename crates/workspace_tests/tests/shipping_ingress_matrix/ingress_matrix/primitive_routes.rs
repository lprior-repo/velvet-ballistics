//! Per-family source primitives over the real Unix-socket binary IPC route,
//! followed by drop/reopen/recovery classification of the recorded outcome.

#![forbid(unsafe_code)]

use std::io::Write;
use std::time::Duration;

use vb_core::value::Taint;
use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotValue};
use vb_ipc::server::{IpcResponse, IpcServer, WorkflowResolver};
use vb_ipc::{IpcCommand, IpcPayload, SubmitRunPayload};
use vb_runtime::recovery::RuntimeRecoveryProduct;

use super::{Cleanup, HEADER, Resolver, encode_request, read_response, socket_path};
use crate::support::{
    Durability, TestResult, assert_ordering, compile, events, open_runtime, persist_artifact,
    terminal, tick_until_terminal,
};

#[path = "primitive_routes/effects.rs"]
mod effects;

#[derive(Clone, Copy)]
enum Drive {
    Terminal,
}

struct Case {
    name: &'static str,
    yaml: String,
    input: SlotValue,
    drive: Drive,
    value: SlotValue,
    taint: Taint,
}

fn input(bytes: SlotValue) -> TestResult<Vec<u8>> {
    Ok(postcard::to_allocvec(&Box::<[SlotValue]>::from([bytes]))?)
}

fn cases() -> Vec<Case> {
    let seed = "  - id: seed\n    set: {output: seed, value: '42'}\n";
    vec![
        Case {
            name: "choose",
            yaml: format!(
                "{HEADER}inputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n{seed}  - id: flag\n    set: {{output: go, value: 'true'}}\n  - id: route\n    choose:\n      branches:\n        - when: '$step.flag'\n          steps:\n            - id: took_a\n              set: {{output: a, value: '7'}}\n      otherwise: done\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(42),
            taint: Taint::Clean,
        },
        Case {
            name: "for_each",
            yaml: format!(
                "{HEADER}vars:\n  entries: [1, 2, 3]\ninputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n{seed}  - id: walk\n    for_each:\n      input: '$vars.entries'\n      variable: entry\n      at_once: 1\n      steps:\n        - id: body\n          set: {{output: seen, value: '$entry'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(42),
            taint: Taint::Clean,
        },
        Case {
            name: "together",
            yaml: format!(
                "{HEADER}inputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n{seed}  - id: both\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_value\n              set: {{output: left, value: '7'}}\n        - label: right\n          steps:\n            - id: right_value\n              set: {{output: right, value: '9'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(42),
            taint: Taint::Clean,
        },
        Case {
            name: "collect",
            yaml: format!(
                "{HEADER}vars:\n  entries: [1, 2, 3]\ninputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n{seed}  - id: gathered\n    collect:\n      variable: batch\n      source: '$vars.entries'\n      pages: 2\n      items: 2\n      steps:\n        - id: body\n          set: {{output: copied, value: 'sum($batch)'}}\n  - id: total\n    set: {{output: total, value: 'sum($step.gathered)'}}\n  - id: done\n    finish: {{result: '$step.total'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(6),
            taint: Taint::Clean,
        },
        Case {
            name: "reduce",
            yaml: format!(
                "{HEADER}vars:\n  entries: [1, 2, 3]\ninputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n{seed}  - id: folded\n    reduce:\n      variable: total\n      input: '$vars.entries'\n      initial: '0'\n      steps:\n        - id: body\n          set: {{output: running, value: '$total + 1'}}\n  - id: done\n    finish: {{result: '$step.folded'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(3),
            taint: Taint::Clean,
        },
        Case {
            name: "repeat",
            yaml: format!(
                "{HEADER}inputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n{seed}  - id: again\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: body\n          set: {{output: tick, value: '7'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(42),
            taint: Taint::Clean,
        },
        Case {
            name: "on_error",
            yaml: format!(
                "{HEADER}inputs:\n  route_tag: {{is: number, max: 8}}\nsteps:\n  - id: seed\n    set: {{output: zero, value: '0'}}\n  - id: failing\n    on_error: {{handler: recovered}}\n    set: {{output: quotient, value: '1 / $step.seed'}}\n  - id: recovered\n    set: {{output: recovered, value: '7'}}\n  - id: done\n    finish: {{result: '$step.recovered'}}\n"
            ),
            input: SlotValue::I64(2),
            drive: Drive::Terminal,
            value: SlotValue::I64(7),
            taint: Taint::Clean,
        },
    ]
}

fn drive_case(
    runtime: &mut vb_runtime::runtime::Runtime,
    run: RunId,
    _workflow: &CompiledWorkflow,
    case: &Case,
) -> TestResult<()> {
    match case.drive {
        Drive::Terminal => {
            tick_until_terminal(runtime, run, 256)?;
            Ok(())
        }
    }
}

fn assert_family_effects(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    workflow: &CompiledWorkflow,
    case: &Case,
) -> TestResult {
    let expected: &[(&str, usize)] = match case.name {
        "choose" => &[("took_a", 1)],
        "for_each" => &[("body", 3)],
        "together" => &[("left_value", 1), ("right_value", 1)],
        "collect" => &[("body", 2)],
        "reduce" => &[("folded", 1)],
        "repeat" => &[("body", 2)],
        "on_error" => &[("recovered", 1)],
        _ => return Err(format!("missing effect oracle for {}", case.name).into()),
    };
    for (name, expected_count) in expected {
        let step = workflow
            .to_parts()
            .step_names
            .iter()
            .position(|candidate| candidate.as_ref() == *name)
            .and_then(|index| u16::try_from(index).ok())
            .ok_or_else(|| format!("{} missing effect step {name}", case.name))?;
        let actual_count = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    vb_storage::JournalEvent::StepSucceeded {
                        run: actual,
                        step: actual_step,
                        ..
                    } if *actual == run && actual_step.get() == step
                )
            })
            .count();
        assert_eq!(actual_count, *expected_count, "{} {name}", case.name);
    }
    effects::assert_family_effects(events, run, workflow, case.name)
}

#[test]
fn binary_ipc_every_source_family_cold_recovery() -> TestResult {
    for case in cases() {
        let workflow = compile(&case.yaml)?;
        let dir = tempfile::tempdir()?;
        let run = RunId::new(400);
        let socket = socket_path("family")?;
        let _cleanup = Cleanup(socket.clone());
        let mut open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        let mut resolver = Resolver::new(workflow.clone());
        let contracts = resolver.resolve_action_contracts(&workflow);
        persist_artifact(
            &open.journal,
            &workflow,
            RuntimePolicy::Strict,
            contracts.as_ref(),
        )?;
        let mut server = IpcServer::bind(&socket)?;
        let mut client = std::os::unix::net::UnixStream::connect(&socket)?;
        client.set_read_timeout(Some(Duration::from_millis(100)))?;
        server.poll_once_with_resolver(
            &mut open.runtime,
            Some(Duration::from_millis(100)),
            Some(&mut resolver),
        )?;
        let payload = IpcPayload::SubmitRunInline(SubmitRunPayload {
            run_id: run,
            workflow: workflow.digest(),
            input: input(case.input)?,
        });
        let bytes = postcard::to_allocvec(&payload)?;
        client.write_all(&encode_request(
            IpcCommand::SubmitRunInline,
            0x6000,
            &bytes,
        )?)?;
        client.flush()?;
        let (header, response) =
            read_response(&mut server, &mut open.runtime, &mut resolver, &mut client)?;
        assert_eq!(header.correlation, 0x6000);
        assert_eq!(
            response,
            Some(IpcResponse::AcceptedRun { run_id: run.get() }),
            "{} submit",
            case.name
        );
        drive_case(&mut open.runtime, run, &workflow, &case)?;
        let durable = events(&open.journal, run)?;
        assert_family_effects(&durable, run, &workflow, &case)?;
        let result = terminal(&durable, run)?;
        assert_eq!(result.value, case.value, "{} value", case.name);
        assert_eq!(result.taint, case.taint, "{} taint", case.name);
        assert_ordering(&durable, run)?;
        drop(client);
        drop(server);
        drop(open);

        let open = open_runtime(dir.path(), RuntimePolicy::Strict, Durability::Strict)?;
        assert_eq!(
            events(&open.journal, run)?,
            durable,
            "{} journal mutated",
            case.name
        );
        let reason = "terminal_state";
        match open.runtime.recover_product(run)? {
            RuntimeRecoveryProduct::CannotResume(product) => {
                assert_eq!(product.reason(), reason, "{}", case.name);
            }
            other => {
                return Err(format!("{} recovered as {other:?}", case.name).into());
            }
        }
        assert_eq!(
            open.runtime.recover_and_resume(run),
            Err(vb_runtime::RuntimeError::RecoveryCannotResume {
                reason: String::from(reason),
            }),
            "{}",
            case.name
        );
    }
    Ok(())
}
