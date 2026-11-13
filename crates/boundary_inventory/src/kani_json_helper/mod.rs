mod operations;
mod types;

use operations::{execute, parse_operation};
use std::ffi::OsString;
use std::io::{self, Write};
use types::HelperError;

fn arguments() -> Result<Vec<String>, HelperError> {
    std::env::args_os()
        .skip(1)
        .map(|argument: OsString| {
            argument
                .into_string()
                .map_err(|_| HelperError::InvalidArgumentEncoding)
        })
        .collect()
}

fn report_error(error: &HelperError) -> Result<(), HelperError> {
    writeln!(io::stderr().lock(), "kani-json-helper: {error}").map_err(HelperError::Diagnostic)
}

fn error_status(error: &HelperError, status: i32) -> i32 {
    match report_error(error) {
        Ok(()) => status,
        Err(_diagnostic_error) => 1,
    }
}

fn write_output(output: Option<String>) -> Result<(), HelperError> {
    if let Some(line) = output {
        writeln!(io::stdout(), "{line}").map_err(HelperError::Output)?;
    }
    Ok(())
}

pub(crate) fn run() -> i32 {
    let result = arguments()
        .and_then(|args| parse_operation(&args))
        .and_then(execute)
        .and_then(write_output);
    match result {
        Ok(()) => 0,
        Err(error @ HelperError::Usage) => error_status(&error, 2),
        Err(error) => error_status(&error, 1),
    }
}
