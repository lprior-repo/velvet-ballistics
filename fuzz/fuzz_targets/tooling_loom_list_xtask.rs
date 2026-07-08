// Fuzz target F04: loom-list.sh xtask output parser
// Fuzzes the xtask output parser with arbitrary output, missing model names,
// partial truncation, unicode, and encoding edge cases.
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::io::Write;
use std::process::Command;

fuzz_target!(|data: &[u8]| {
    // Simulate xtask loom output with arbitrary content
    let mut tmp = match tempfile::NamedTempFile::new() {
        Ok(f) => f,
        Err(_) => return,
    };
    if tmp.write_all(data).is_err() {
        return;
    }
    if tmp.flush().is_err() {
        return;
    }

    // Create a fake xtask that outputs the fuzz data
    let tmp_path = tmp.path().to_string_lossy().to_string();
    let mut fake_xtask = match tempfile::NamedTempFile::new() {
        Ok(f) => f,
        Err(_) => return,
    };
    let script = format!("#!/usr/bin/env bash\ncat '{}'\nexit 1\n", tmp_path);
    if fake_xtask.write_all(script.as_bytes()).is_err() {
        return;
    }
    if fake_xtask.flush().is_err() {
        return;
    }
    let fake_path = fake_xtask.path().to_string_lossy().to_string();
    match std::process::Command::new("chmod")
        .arg("+x")
        .arg(&fake_path)
        .output()
    {
        Ok(_) | Err(_) => {}
    }

    // Replace the cargo xtask loom invocation with our fake
    // We do this by creating a wrapper that the script can use
    let mut wrapper = match tempfile::NamedTempFile::new() {
        Ok(f) => f,
        Err(_) => return,
    };
    let wrapper_script = format!(
        "#!/usr/bin/env bash\nif [ \"$1\" = \"xtask\" ] && [ \"$2\" = \"loom\" ]; then\n  exec '{}'\nfi\nexit 1\n",
        fake_path
    );
    if wrapper.write_all(wrapper_script.as_bytes()).is_err() {
        return;
    }
    if wrapper.flush().is_err() {
        return;
    }
    let wrapper_path = wrapper.path().to_string_lossy().to_string();
    match std::process::Command::new("chmod")
        .arg("+x")
        .arg(&wrapper_path)
        .output()
    {
        Ok(_) | Err(_) => {}
    }

    // Run loom-list.sh with the fake cargo in PATH
    let wrapper_dir = std::path::Path::new(&wrapper_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "/tmp".to_string());
    match Command::new("bash")
        .arg("scripts/loom-list.sh")
        .env("PATH", format!("{}:/usr/bin:/bin", wrapper_dir))
        .output()
    {
        Ok(_) | Err(_) => {}
    }
});
