#![forbid(unsafe_code)]
#[path = "vb_f1_do_guards/boundary.rs"]
mod guard_boundary;
#[path = "vb_f1_do_guards/journal.rs"]
mod guard_journal;
#[path = "vb_f1_do_guards/runtime.rs"]
mod guard_runtime;
#[path = "vb_f1_do_guards/support.rs"]
mod guard_support;

#[test]
fn guarded_rollback_smoke_passes() -> Result<(), String> {
    guard_runtime::run_scenario(true)
}

#[test]
fn guarded_rollback_smoke_fails() -> Result<(), String> {
    guard_runtime::run_scenario(false)
}
