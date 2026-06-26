// Fuzz target for wait digest field sensitivity (PO-003).
// Bead: vb-xi2f.32
// Verifier: cargo-fuzz
// Command: cargo fuzz run wait_digest_sensitivity -- -max_len=64 -max_total_time=120
//
// Verifies that different wait event/timeout values
// produce different WorkflowDigest values. Mutates event
// and timeout bytes; maps them to integer-like strings
// that pass the YAML validator.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let event_num = u16::from(data[0]);
    let timeout_num = u16::from(data[1]);
    let alt_event_num = data.get(2).copied().map(u16::from).unwrap_or(event_num.wrapping_add(1));
    let alt_timeout_num = data.get(3).copied().map(u16::from).unwrap_or(timeout_num.wrapping_add(1));

    let event = event_num.to_string();
    let timeout = timeout_num.to_string();

    // Build two different workflows and verify digests differ
    let alt_event_str = alt_event_num.to_string();
    let alt_timeout_str = alt_timeout_num.to_string();
    let src_a = fuzz_lib_build_workflow(Some(&event), Some(&timeout));
    let src_b = fuzz_lib_build_workflow(
        if alt_event_num != event_num { Some(&alt_event_str) } else { None },
        Some(&alt_timeout_str),
    );

    let parsed_a = vb_compile::parse_workflow_source(&src_a);
    let parsed_b = vb_compile::parse_workflow_source(&src_b);
    let (Ok(source_a), Ok(source_b)) = (parsed_a, parsed_b) else { return; };

    let compiled_a = vb_compile::compile_source(&source_a);
    let compiled_b = vb_compile::compile_source(&source_b);
    let (Ok(wf_a), Ok(wf_b)) = (compiled_a, compiled_b) else { return; };

    let digest_a = wf_a.digest();
    let digest_b = wf_b.digest();

    // If the two configurations are different, digests must differ
    if source_a != source_b {
        assert!(
            digest_a != digest_b,
            "COLLISION: different wait configs produced same digest"
        );
    }
});

fn fuzz_lib_build_workflow(event: Option<&str>, timeout: Option<&str>) -> String {
    let mut wait = String::from("  - id: w\n    wait:");
    if let Some(e) = event {
        wait.push_str(&format!("\n      event: \"{e}\""));
    }
    if let Some(t) = timeout {
        wait.push_str(&format!("\n      timeout: \"{t}\""));
    }
    format!("version: velvet-ballastics/v1\nname: fuzz-wait\nwhen:\n  manual: {{}}\nsteps:\n{wait}\n  - id: d\n    finish:\n      result: 0\n")
}
