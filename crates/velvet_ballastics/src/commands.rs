//! Command implementations for velvet-ballastics.
//! Thin re-export facade over sibling modules: run, storage, bench.

// Re-export EmitTarget so callers can use it via commands::
pub use crate::args::EmitTarget;

// Re-export run commands
pub use crate::run::{
    cmd_compile, cmd_run, cmd_run_compiled, cmd_validate, map_runtime_inputs,
    INPUT_MAPPING_DECODE_FAILED_MESSAGE, INPUT_MAPPING_SLOT_COUNT_EXCEEDED_MESSAGE,
    INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
};

// Re-export storage commands
pub use crate::storage::{
    cmd_events, cmd_inspect, cmd_ipc_serve, cmd_replay, print_event, event_name,
    StorageWorkflowResolver,
};

// Re-export bench commands
pub use crate::bench::{cmd_bench_run, cmd_doctor};
