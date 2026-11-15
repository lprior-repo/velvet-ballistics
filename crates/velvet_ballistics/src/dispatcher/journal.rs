use super::{Command, Route};
use std::ops::ControlFlow::{Break, Continue};

pub(super) fn reading(command: Command) -> Route {
    match command {
        Command::Inspect { run_id, db, output } => {
            Break(crate::inspect::cmd_inspect(&run_id, &db, output))
        }
        Command::Events {
            run_id,
            db,
            output,
            status,
            limit,
        } => Break(crate::events::cmd_events(
            &run_id, &db, output, status, limit,
        )),
        other => Continue(other),
    }
}

pub(super) fn tracing(command: Command) -> Route {
    match command {
        Command::Trace {
            run_id,
            db,
            output,
            filters,
        } => Break(crate::trace::cmd_trace(&run_id, &db, output, filters)),
        other => Continue(other),
    }
}

pub(super) fn replay(command: Command) -> Route {
    match command {
        Command::Replay { run_id, db, output } => {
            Break(crate::replay::cmd_replay(&run_id, &db, output))
        }
        Command::Resume { run_id, db, output } => {
            Break(crate::replay::cmd_resume(&run_id, &db, output))
        }
        other => Continue(other),
    }
}

pub(super) fn retry(command: Command) -> Route {
    match command {
        Command::Retry {
            run_id,
            step,
            db,
            output,
        } => Break(crate::run_ops::cmd_retry(&run_id, step, &db, output)),
        other => Continue(other),
    }
}

pub(super) fn answer(command: Command) -> Route {
    match command {
        Command::Answer {
            run_id,
            selector,
            value_file,
            db,
            output,
        } => Break(crate::run_cancel::cmd_answer(
            &run_id,
            selector,
            &value_file,
            &db,
            output,
        )),
        other => Continue(other),
    }
}

pub(super) fn cancel(command: Command) -> Route {
    match command {
        Command::Cancel {
            run_id,
            db,
            reason,
            output,
        } => Break(crate::run_cancel_ops::cmd_cancel(
            &run_id, &db, reason, output,
        )),
        other => Continue(other),
    }
}

pub(super) fn maintenance(command: Command) -> Route {
    match command {
        Command::Doctor { db, output } => Break(crate::doctor::cmd_doctor(db.as_deref(), output)),
        other => Continue(other),
    }
}

pub(super) fn incidents(command: Command) -> Route {
    match command {
        Command::Diff {
            run_a,
            run_b,
            db,
            output,
        } => Break(crate::incident_ops::cmd_diff(&run_a, &run_b, &db, output)),
        Command::Incident { run_id, db, output } => {
            Break(crate::incident_diff::cmd_incident(&run_id, &db, output))
        }
        other => Continue(other),
    }
}
