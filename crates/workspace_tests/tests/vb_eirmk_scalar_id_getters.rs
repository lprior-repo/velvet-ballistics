#![forbid(unsafe_code)]

use vb_core::{RunId, WorkflowId};

const WORKFLOW_ONE: WorkflowId = WorkflowId::new(1);
const WORKFLOW_ONE_ALIAS: u32 = WORKFLOW_ONE.as_u32();
const WORKFLOW_ONE_GET: u32 = WORKFLOW_ONE.get();
const RUN_ONE: RunId = RunId::new(1);
const RUN_ONE_ALIAS: u64 = RUN_ONE.as_u64();
const RUN_ONE_GET: u64 = RUN_ONE.get();

fn assert_workflow_value(id: WorkflowId, expected: u32) {
    assert_eq!(id.as_u32(), expected);
    assert_eq!(id.get(), expected);
}

fn assert_run_value(id: RunId, expected: u64) {
    assert_eq!(id.as_u64(), expected);
    assert_eq!(id.get(), expected);
}

fn assert_workflow_wire(id: WorkflowId) -> Result<(), String> {
    let typed = postcard::to_allocvec(&id).map_err(|error| error.to_string())?;
    let raw = postcard::to_allocvec(&id.as_u32()).map_err(|error| error.to_string())?;
    assert_eq!(typed, raw);
    let decoded = postcard::from_bytes::<WorkflowId>(&typed).map_err(|error| error.to_string())?;
    assert_eq!(decoded, id);
    Ok(())
}

fn assert_run_wire(id: RunId) -> Result<(), String> {
    let typed = postcard::to_allocvec(&id).map_err(|error| error.to_string())?;
    let raw = postcard::to_allocvec(&id.as_u64()).map_err(|error| error.to_string())?;
    assert_eq!(typed, raw);
    let decoded = postcard::from_bytes::<RunId>(&typed).map_err(|error| error.to_string())?;
    assert_eq!(decoded, id);
    Ok(())
}

#[test]
fn scenario_workflow_id_alias_preserves_boundary_values_and_const_use() {
    // Given: WorkflowId supports its complete u32 domain.
    // When: its alias is used at zero, one, and the maximum.
    assert_workflow_value(WorkflowId::new(0), 0);
    assert_workflow_value(WORKFLOW_ONE, 1);
    assert_workflow_value(WorkflowId::new(u32::MAX), u32::MAX);
    // Then: compile-time alias and existing getter values agree with literal one.
    assert_eq!(WORKFLOW_ONE_ALIAS, 1);
    assert_eq!(WORKFLOW_ONE_GET, 1);
}

#[test]
fn scenario_run_id_alias_preserves_boundary_values_and_const_use() {
    // Given: RunId supports its complete u64 domain.
    // When: its alias is used at zero, one, and the maximum.
    assert_run_value(RunId::new(0), 0);
    assert_run_value(RUN_ONE, 1);
    assert_run_value(RunId::new(u64::MAX), u64::MAX);
    // Then: compile-time alias and existing getter values agree with literal one.
    assert_eq!(RUN_ONE_ALIAS, 1);
    assert_eq!(RUN_ONE_GET, 1);
}

#[test]
fn scenario_workflow_id_wire_bytes_remain_unchanged() -> Result<(), String> {
    // Given: WorkflowId values at zero, one, and u32 maximum.
    // When: each typed ID is serialized and decoded.
    assert_workflow_wire(WorkflowId::new(0))?;
    assert_workflow_wire(WorkflowId::new(1))?;
    assert_workflow_wire(WorkflowId::new(u32::MAX))?;
    // Then: the helper proves typed bytes equal raw u32 bytes and decode exactly.
    Ok(())
}

#[test]
fn scenario_run_id_wire_bytes_remain_unchanged() -> Result<(), String> {
    // Given: RunId values at zero, one, and u64 maximum.
    // When: each typed ID is serialized and decoded.
    assert_run_wire(RunId::new(0))?;
    assert_run_wire(RunId::new(1))?;
    assert_run_wire(RunId::new(u64::MAX))?;
    // Then: the helper proves typed bytes equal raw u64 bytes and decode exactly.
    Ok(())
}
