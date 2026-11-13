// BIN unit tests for write errors.
// Uses real report_success/report_results through generic Write.

use super::checks::{Check, Oracle};
use super::{report_results, report_success};
use std::io::Write;

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buf: &[u8]) -> Result<usize, std::io::Error> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "write failed",
        ))
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

#[test]
fn report_success_failing_write_returns_error() {
    let mut writer = FailingWriter;
    let result = report_success(
        &Check {
            type_name: "TestError",
            enum_path: "path/to/enum.rs",
            oracles: &[],
        },
        &Oracle {
            path: "path/to/oracle.rs",
            function: "test_fn",
        },
        3,
        &mut writer,
    );
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("write error"),
        "Expected write error message. Got: {}",
        err_msg
    );
}

#[test]
fn report_results_failing_write_returns_error() {
    let mut writer = FailingWriter;
    let result = report_results(&mut writer, &[]);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("write error"),
        "Expected write error message. Got: {}",
        err_msg
    );
}
