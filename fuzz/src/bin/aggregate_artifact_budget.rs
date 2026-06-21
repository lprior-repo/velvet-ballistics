//! Fuzz target: aggregate_artifact_budget.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_aggregate_artifact_budget)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}

#[cfg(feature = "fuzz")]
fn fuzz_aggregate_artifact_budget(data: &[u8]) {
    let Ok(inventory) = vb_boundary_inventory::boundary_inventory::parser::parse_inventory(data)
    else {
        return;
    };
    let total: u64 = inventory.records.len().min(u64::MAX as usize) as u64;
    for record in &inventory.records {
        let _ = record.evidence.clone();
        let _ = total;
    }
}