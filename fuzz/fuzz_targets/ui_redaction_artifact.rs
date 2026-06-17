#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let artifact = String::from_utf8_lossy(data);
    // scan_release_artifact must never panic and must return a typed Result.
    // This assertion fires for ALL inputs, not just sentinel-containing ones.
    let result = vb_ui_snapshot::redaction::scan_release_artifact(&artifact);
    // Verify result is a valid Result enum (never panics, always typed)
    let _is_ok = result.is_ok();

    // Targeted assertion: sentinel-containing artifacts must be rejected
    if artifact.contains("vb_nf2u_secret_sentinel") {
        assert!(
            result.is_err(),
            "artifact containing secret sentinel must be rejected"
        );
    }
});
