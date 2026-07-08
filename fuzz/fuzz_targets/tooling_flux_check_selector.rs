// Fuzz target F02: flux-check-package.sh selector rejection
// Fuzzes the selector loop with arbitrary flag-like arguments, unicode,
// case variations, and injection attempts.
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::process::Command;

fn utf8_prefix(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let mut end = 0usize;
    for (idx, ch) in input.char_indices() {
        let Some(next) = idx.checked_add(ch.len_utf8()) else {
            break;
        };
        if next > max_bytes {
            break;
        }
        end = next;
    }
    input.get(..end).unwrap_or("")
}

fuzz_target!(|data: &[u8]| {
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };
    let input = utf8_prefix(input, 4096);

    // Generate arguments: first arg is a package-like name, rest are selector-like flags
    let parts: Vec<&str> = input.split_whitespace().take(32).collect();
    if parts.is_empty() {
        return;
    }

    let Some((package_arg, selector_args)) = parts.split_first() else {
        return;
    };

    let mut full_args = Vec::new();
    if full_args.try_reserve(parts.len()).is_err() {
        return;
    }
    full_args.push(*package_arg);
    full_args.extend(selector_args.iter().copied());

    match Command::new("bash")
        .arg("scripts/flux-check-package.sh")
        .args(&full_args)
        .output()
    {
        Ok(_) | Err(_) => {}
    }
});
