#[path = "../../../boundary_inventory/src/verus_registry_targets/mod.rs"]
mod verus_registry_targets;

use std::env;
use std::io;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut output = io::stdout().lock();
    let mut diagnostics = io::stderr().lock();
    verus_registry_targets::run(env::args_os().skip(1), &mut output, &mut diagnostics)
}
