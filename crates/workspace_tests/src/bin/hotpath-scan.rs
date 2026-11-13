use std::io::{self, Write};
use std::process::ExitCode;

#[path = "../source_validators/text.rs"]
mod source_text;

#[path = "../hotpath_scan/mod.rs"]
mod hotpath_scan;

fn main() -> ExitCode {
    let result = std::env::current_dir()
        .map_err(hotpath_scan::Failure::from)
        .and_then(|root| hotpath_scan::run(&root, &mut io::stdout().lock()));
    match result {
        Ok(code) => ExitCode::from(code),
        Err(error) => match writeln!(io::stderr().lock(), "{error}") {
            Ok(()) => ExitCode::from(error.exit_code()),
            Err(_) => ExitCode::from(1),
        },
    }
}
