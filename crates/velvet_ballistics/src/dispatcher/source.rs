use super::{Command, Route};
use std::ops::ControlFlow::{Break, Continue};

pub(super) fn analysis(command: Command) -> Route {
    match command {
        Command::Verify {
            workflow,
            profile,
            output,
        } => Break(crate::verify::cmd_verify(&workflow, profile, output)),
        Command::Validate { workflow, output } => {
            Break(crate::validate::cmd_validate(&workflow, output))
        }
        Command::Explain { workflow, output } => {
            Break(crate::explain::cmd_explain(&workflow, output))
        }
        other => Continue(other),
    }
}

pub(super) fn artifacts(command: Command) -> Route {
    match command {
        Command::Compile {
            workflow,
            emit,
            out,
            output,
        } => Break(crate::compile::cmd_compile(&workflow, emit, &out, output)),
        Command::Graph { workflow, output } => Break(crate::graph::cmd_graph(&workflow, output)),
        other => Continue(other),
    }
}

pub(super) fn simulations(command: Command) -> Route {
    match command {
        Command::BenchRun { workflow, output } => {
            Break(crate::bench_run::cmd_bench_run(&workflow, output))
        }
        Command::Simulate { workflow, output } => {
            Break(crate::simulate::cmd_simulate(&workflow, output))
        }
        other => Continue(other),
    }
}

pub(super) fn comparison(command: Command) -> Route {
    match command {
        Command::WorkflowDiff {
            workflow,
            against,
            db,
            output,
        } => Break(crate::semantic_diff::cmd_workflow_diff(
            &workflow,
            against.as_deref(),
            db.as_deref(),
            output,
        )),
        other => Continue(other),
    }
}
