// Fuzz target F02: flux-check-package.sh selector rejection
// Fuzzes the selector loop with arbitrary flag-like arguments, unicode,
// case variations, and injection attempts.
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::process::Command;

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };
    let input = if input.len() > 4096 { &input[..4096] } else { input };

    // Generate arguments: first arg is a package-like name, rest are selector-like flags
    let parts: Vec<&str> = input.split_whitespace().take(32).collect();
    if parts.is_empty() {
        return;
    }

    let package_arg = parts[0];
    let selector_args: Vec<&str> = if parts.len() > 1 {
        parts[1..].to_vec()
    } else {
        vec![]
    };

    let mut full_args = vec![package_arg];
    full_args.extend(selector_args);

    let _ = Command::new("bash")
        .arg("scripts/flux-check-package.sh")
        .args(&full_args)
        .output();
});
