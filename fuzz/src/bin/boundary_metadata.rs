//! Fuzz target: boundary_metadata.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_boundary_metadata)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}

#[cfg(feature = "fuzz")]
fn fuzz_boundary_metadata(data: &[u8]) {
    let Ok(inventory) = vb_boundary_inventory::boundary_inventory::parser::parse_inventory(data)
    else {
        return;
    };
    let _ = inventory.schema_version;
    for record in &inventory.records {
        let _ = record.id.clone();
        let _ = record.class;
    }
}