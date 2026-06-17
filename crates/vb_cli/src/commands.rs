#![forbid(unsafe_code)]
//! Command implementations for velvet-ballistics.
//! Thin re-export facade over sibling modules: run, storage, bench.

// Re-export EmitTarget so callers can use it via commands::
pub(crate) use crate::args::EmitTarget;

// Re-export run commands
pub(crate) use crate::run::{
    INPUT_MAPPING_DECODE_FAILED_MESSAGE, INPUT_MAPPING_EMPTY_INPUT_BIN_MESSAGE,
    INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE, INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
    cmd_compile, cmd_run, cmd_run_compiled, cmd_validate, map_runtime_inputs,
};

// Re-export storage commands
pub(crate) use crate::storage::{
    StorageWorkflowResolver, cmd_events, cmd_inspect, cmd_ipc_serve, cmd_replay, event_name,
    print_event,
};

// Re-export bench commands
pub(crate) use crate::bench::{cmd_bench_run, cmd_doctor};
