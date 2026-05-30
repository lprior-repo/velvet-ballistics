// Fuzz target for wait sentinel collision detection (PO-007).
// Bead: vb-xi2f.32
// Verifier: cargo-fuzz
// Command: cargo fuzz run wait_sentinel_collision -- -max_len=64 -max_total_time=120
//
// Verifies that different timeout values produce different digests.
// The sentinel "none" cannot reach canonical_digest through YAML
// compilation (validator requires integer strings), so we verify
// the reachable property: WaitEvent with timeout=a != WaitEvent with
// timeout=b for integer a,b where a != b.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    let event_num = u16::from(data[0]);
    let timeout_a = u16::from(data[1]);
    let timeout_b = u16::from(data[2]);

    // Only relevant when the two timeout values differ
    if timeout_a == timeout_b {
        return;
    }

    let event = event_num.to_string();
    let ta = timeout_a.to_string();
    let tb = timeout_b.to_string();

    let src_a = build_workflow(Some(&event), Some(&ta));
    let src_b = build_workflow(Some(&event), Some(&tb));

    let parsed_a = vb_yaml::parse_workflow_source(&src_a);
    let parsed_b = vb_yaml::parse_workflow_source(&src_b);
    let (Ok(source_a), Ok(source_b)) = (parsed_a, parsed_b) else { return; };

    let compiled_a = vb_compile::compile_source(&source_a);
    let compiled_b = vb_compile::compile_source(&source_b);
    let (Ok(wf_a), Ok(wf_b)) = (compiled_a, compiled_b) else { return; };

    assert!(
        wf_a.digest() != wf_b.digest(),
        "SENTINEL COLLISION: different timeout values (\"{ta}\" vs \"{tb}\") produced same digest",
    );
});

fn build_workflow(event: Option<&str>, timeout: Option<&str>) -> String {
    let mut wait = String::from("  - id: w\n    wait:");
    if let Some(e) = event {
        wait.push_str(&format!("\n      event: \"{e}\""));
    }
    if let Some(t) = timeout {
        wait.push_str(&format!("\n      timeout: \"{t}\""));
    }
    format!("version: velvet-ballastics/v1\nname: fuzz-wait\nwhen:\n  manual: {{}}\nsteps:\n{wait}\n  - id: d\n    finish:\n      result: 0\n")
}
