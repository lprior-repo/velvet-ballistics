// Fuzz target: fuzz_choose_when_parse.rs
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO: PO-FUZZ-001 — Fuzz slot_from_text / compile_workflow with arbitrary when strings
// Command: cargo fuzz run fuzz_choose_when_parse -- -max_len=256 -runs=100000
//
// GOD RULE 2: Binds to production compile_workflow and the choose lowering path.

#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;

/// Build a YAML workflow with a choose step whose when strings come from fuzzer input.
fn fuzz_choose_when(when_strings: &[String]) {
    let mut yaml = String::from(
        "version: velvet-ballistics/v1\nname: fuzz\nwhen:\n  manual: {}\nsteps:\n",
    );
    yaml.push_str("  - id: pick\n    choose:\n      branches:\n");

    let count = when_strings.len().min(64);
    for (i, when) in when_strings.iter().take(count).enumerate() {
        // Sanitize the when string for YAML embedding
        let safe_when = when.replace('\"', "\\\"");
        yaml.push_str(&format!(
            "        - when: \"{}\"\n          steps:\n",
            safe_when
        ));
        // Add a body set step
        yaml.push_str(&format!(
            "            - id: b{i}\n              set:\n                output: out_{i}\n                value: \"1\"\n"
        ));
    }
    yaml.push_str("      otherwise: done\n");
    yaml.push_str("  - id: done\n    finish:\n      result: \"ok\"\n");

    // compile_workflow must never panic
    let _ = vb_compile::compile_workflow(yaml.as_bytes());
}

fuzz_target!(|data: &[u8]| {
    // Interpret arbitrary bytes as when strings
    let mut when_strings: Vec<String> = Vec::new();
    let mut current = String::new();
    for &byte in data.iter().take(1024) {
        if byte == 0 {
            if !current.is_empty() {
                when_strings.push(std::mem::take(&mut current));
            }
        } else {
            current.push(byte as char);
        }
    }
    if !current.is_empty() {
        when_strings.push(current);
    }
    if when_strings.is_empty() {
        when_strings.push("0".to_string());
    }
    fuzz_choose_when(&when_strings);
});
