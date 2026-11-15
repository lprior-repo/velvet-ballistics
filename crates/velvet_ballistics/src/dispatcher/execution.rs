use super::{Command, Route};
use std::ops::ControlFlow::{Break, Continue};

pub(super) fn source(command: Command) -> Route {
    match command {
        Command::Run {
            workflow,
            input_bin,
            durability,
            db,
            step,
            output,
        } => Break(match step {
            Some(target) => crate::run_step::cmd_run_step(&workflow, durability, &target, output),
            None => crate::run::cmd_run(&workflow, &input_bin, durability, db.as_deref(), output),
        }),
        other => Continue(other),
    }
}

pub(super) fn artifact(command: Command) -> Route {
    match command {
        Command::RunCompiled {
            workflow,
            input_bin,
            durability,
            db,
            output,
        } => Break(crate::run_compiled::cmd_run_compiled(
            &workflow,
            &input_bin,
            durability,
            db.as_deref(),
            output,
        )),
        other => Continue(other),
    }
}

pub(super) fn submission(command: Command) -> Route {
    match command {
        Command::Submit {
            workflow,
            input_bin,
            db,
            durability,
            output,
        } => Break(crate::submit::cmd_submit(
            &workflow, &input_bin, &db, durability, output,
        )),
        other => Continue(other),
    }
}

pub(super) fn server(command: Command) -> Route {
    match command {
        Command::IpcServe { socket, db } => Break(crate::ipc_serve::cmd_ipc_serve(&socket, &db)),
        other => Continue(other),
    }
}
