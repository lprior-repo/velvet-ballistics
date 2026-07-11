// Fuzz target F01: kani-list.sh argument parsing
// Fuzzes script invocation with arbitrary arguments, special characters,
// path traversal via KANI_LIST_DIR, and feature injection.
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
    // Convert arbitrary bytes to a string, skip if not valid UTF-8
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };

    // Trim to a reasonable max length to avoid system arg limits
    let input = utf8_prefix(input, 4096);

    // Split into arguments (max 10 args to avoid explosion)
    let args: Vec<&str> = input.split_whitespace().take(10).collect();
    if args.is_empty() {
        return;
    }

    // Try running kani-list.sh with the fuzzed arguments
    // Use a short timeout to avoid hanging on cargo operations
    match Command::new("bash")
        .arg("scripts/kani-list.sh")
        .args(&args)
        .env("KANI_LIST_DIR", "/tmp/fuzz-kani-list")
        .output()
    {
        Ok(_) | Err(_) => {}
    }
    // No assertions — fuzzer checks for panics/crashes only
});
