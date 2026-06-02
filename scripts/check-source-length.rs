mod source_length_gate;
mod source_length_ledger;
mod source_length_scan;

use std::process::ExitCode;

fn main() -> ExitCode {
    source_length_gate::main_exit()
}
