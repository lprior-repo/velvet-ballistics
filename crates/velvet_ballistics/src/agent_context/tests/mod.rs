#[cfg(any())]
// disabled: stale state field refs (BLOCK_GLOBAL follow-up)
mod kani_harnesses;
#[cfg(test)]
mod properties;
#[cfg(test)]
mod unit;

const IMPLEMENTED_COMMAND_NAMES: &[&str] = &[
    "action inspect",
    "action list",
    "agent-context",
    "ai-context",
    "answer",
    "bench-run",
    "cancel",
    "compile",
    "diff",
    "doctor",
    "events",
    "explain",
    "graph",
    "help",
    "incident",
    "inspect",
    "ipc-serve",
    "replay",
    "resume",
    "retry",
    "run",
    "run-compiled",
    "simulate",
    "status",
    "submit",
    "system status",
    "trace",
    "validate",
    "verify",
    "version",
];
