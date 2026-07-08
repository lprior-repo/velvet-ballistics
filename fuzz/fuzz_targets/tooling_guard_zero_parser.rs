// Fuzz target F03: guard-zero-tests.sh output parser
// Fuzzes the test output parser with arbitrary cargo test stdout/stderr,
// including integer overflow attempts, truncated output, BOM prefixes,
// and binary garbage.
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;
use std::process::Command;

fuzz_target!(|data: &[u8]| {
    // Write fuzz data to a temp file that simulates cargo test output
    let mut tmp = match tempfile::NamedTempFile::new() {
        Ok(f) => f,
        Err(_) => return,
    };

    // Write the fuzz data (binary OK — the parser should handle any bytes)
    if tmp.write_all(data).is_err() {
        return;
    }
    if tmp.flush().is_err() {
        return;
    }

    // Create a script that cats the temp file to simulate cargo test output
    let fake_test_path = {
        let tmp_path = tmp.path().to_string_lossy().to_string();
        let mut fake_test = match tempfile::NamedTempFile::new() {
            Ok(f) => f,
            Err(_) => return,
        };
        let script = format!("#!/usr/bin/env bash\ncat '{}'\nexit 0\n", tmp_path);
        if fake_test.write_all(script.as_bytes()).is_err() {
            return;
        }
        if fake_test.flush().is_err() {
            return;
        }
        // Make executable
        let path = fake_test.path().to_string_lossy().to_string();
        match std::process::Command::new("chmod")
            .arg("+x")
            .arg(&path)
            .output()
        {
            Ok(_) | Err(_) => {}
        }
        path
    };

    // Run guard-zero-tests.sh with the fake test
    match Command::new("bash")
        .arg("scripts/guard-zero-tests.sh")
        .arg("--")
        .arg(&fake_test_path)
        .output()
    {
        Ok(_) | Err(_) => {}
    }
});
