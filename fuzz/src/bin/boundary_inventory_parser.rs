//! Fuzz target: boundary_inventory_parser.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_boundary_inventory_parser)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}

#[cfg(feature = "fuzz")]
fn fuzz_boundary_inventory_parser(data: &[u8]) {
    let _ = vb_boundary_inventory::boundary_inventory::parser::parse_inventory(data);
}