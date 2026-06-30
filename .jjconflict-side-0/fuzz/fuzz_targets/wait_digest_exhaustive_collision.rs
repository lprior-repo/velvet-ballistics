// Fuzz target for exhaustive wait digest collision detection (PO-012).
// Bead: vb-xi2f.32
// Verifier: cargo-fuzz
// Command: cargo fuzz run wait_digest_exhaustive_collision -- -max_len=64 -max_total_time=180
//
// Verifies that all three legal Wait configurations (WaitUntil,
// WaitEvent-bounded, WaitEvent-unbounded) produce pairwise-distinct
// digests for distinct field values. Uses byte input to randomize
// Wait shapes with integer-like event/timeout values.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let byte1 = data[0];
    let byte2 = data[1];
    let event_num1 = u16::from(data[2]);
    let event_num2 = u16::from(data[3]);

    // Map bytes to Wait shapes
    let shape1 = byte1 % 3;
    let shape2 = byte2 % 3;

    let (e1, t1): (Option<String>, Option<String>) = match shape1 {
        0 => (None, Some("10".to_string())),                                   // WaitUntil
        1 => (Some(event_num1.to_string()), None),                              // WaitEvent unbounded
        _ => (Some(event_num1.to_string()), Some("20".to_string())),            // WaitEvent bounded
    };
    let (e2, t2): (Option<String>, Option<String>) = match shape2 {
        0 => (None, Some("10".to_string())),
        1 => (Some(event_num2.to_string()), None),
        _ => (Some(event_num2.to_string()), Some("30".to_string())),
    };

    // If the two configurations are identical, skip
    if e1 == e2 && t1 == t2 {
        return;
    }

    let yaml1 = build_workflow_from_opts(&e1, &t1);
    let yaml2 = build_workflow_from_opts(&e2, &t2);

    let src1 = vb_compile::parse_workflow_source(&yaml1);
    let src2 = vb_compile::parse_workflow_source(&yaml2);
    let (Ok(s1), Ok(s2)) = (src1, src2) else { return; };

    let c1 = vb_compile::compile_source(&s1);
    let c2 = vb_compile::compile_source(&s2);
    let (Ok(w1), Ok(w2)) = (c1, c2) else { return; };

    assert!(
        w1.digest() != w2.digest(),
        "EXHAUSTIVE COLLISION: distinct Wait configs produced same digest (shape1={shape1} shape2={shape2})",
    );
});

fn build_workflow_from_opts(event: &Option<String>, timeout: &Option<String>) -> String {
    let mut wait = String::from("  - id: w\n    wait:");
    if let Some(e) = event {
        wait.push_str(&format!("\n      event: \"{e}\""));
    }
    if let Some(t) = timeout {
        wait.push_str(&format!("\n      timeout: \"{t}\""));
    }
    format!("version: velvet-ballastics/v1\nname: fuzz-wait\nwhen:\n  manual: {{}}\nsteps:\n{wait}\n  - id: d\n    finish:\n      result: 0\n")
}
