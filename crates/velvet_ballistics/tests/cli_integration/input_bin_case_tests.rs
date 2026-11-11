use super::{
    SlotValue, assert_cli_success, cli_tempdir, forced_assertion_failure, input_slot_parts,
    output_stdout, run_cli, write_test_file,
};
use std::path::Path;

pub(super) fn directory() -> Result<tempfile::TempDir, String> {
    match cli_tempdir() {
        Ok(dir) => Ok(dir),
        Err(err) => {
            assert!(forced_assertion_failure(), "tempdir failed: {err}");
            Err(format!("tempdir failed: {err}"))
        }
    }
}

pub(super) fn write_workflow(workflow_path: &Path) -> Result<(), String> {
    let parts =
        input_slot_parts().map_err(|err| format!("failed to construct input slot parts: {err}"))?;
    let workflow_payload = match postcard::to_allocvec(&parts) {
        Ok(payload) => payload,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to encode workflow payload: {err}"
            );
            return Err(format!("failed to encode workflow payload: {err}"));
        }
    };
    if !write_test_file(&workflow_path, &workflow_payload) {
        return Err("failed to write workflow payload".into());
    }
    Ok(())
}

pub(super) fn write_input(input_path: &Path) -> Result<(), String> {
    let values: Box<[SlotValue]> = Box::from([SlotValue::I64(7)]);
    let payload = match postcard::to_allocvec(&values) {
        Ok(payload) => payload,
        Err(err) => {
            assert!(
                forced_assertion_failure(),
                "failed to encode input payload: {err}"
            );
            return Err(format!("failed to encode input payload: {err}"));
        }
    };
    if !write_test_file(&input_path, &payload) {
        return Err("failed to write input payload".into());
    }
    Ok(())
}

pub(super) fn run(workflow_path: &Path, input_path: &Path) -> Result<(), String> {
    let run_output = match run_cli(&[
        std::ffi::OsStr::new("run-compiled"),
        workflow_path.as_os_str(),
        std::ffi::OsStr::new("--input-bin"),
        input_path.as_os_str(),
        std::ffi::OsStr::new("--durability"),
        std::ffi::OsStr::new("none"),
    ]) {
        Some(output) => output,
        None => return Err("failed to execute velvet_ballistics".into()),
    };
    assert_cli_success(&run_output, "run-compiled --durability none with input-bin");
    let run_stdout = output_stdout(&run_output);
    assert!(
        run_stdout.contains("run completed"),
        "run stdout should report completion: {run_stdout}"
    );
    Ok(())
}
