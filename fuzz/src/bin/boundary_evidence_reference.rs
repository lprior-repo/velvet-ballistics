//! Fuzz target: boundary_evidence_reference.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_boundary_evidence_reference)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}

#[cfg(feature = "fuzz")]
fn fuzz_boundary_evidence_reference(data: &[u8]) {
    use vb_boundary_inventory::boundary_inventory::api::{classify_boundary, required_evidence};
    use vb_boundary_inventory::boundary_inventory::types::BoundaryCandidate;

    let Ok(inventory) = vb_boundary_inventory::boundary_inventory::parser::parse_inventory(data)
    else {
        return;
    };
    for record in &inventory.records {
        let candidate = BoundaryCandidate::new(&record.source_path, record.id.clone());
        if let Ok(classified) = classify_boundary(candidate) {
            let _ = required_evidence(classified);
        }
    }
}