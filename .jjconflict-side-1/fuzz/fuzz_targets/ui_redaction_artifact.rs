#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let artifact = String::from_utf8_lossy(data);
    let result = vb_ui_snapshot::redaction::scan_release_artifact(&artifact);
    if artifact.contains("vb_nf2u_secret_sentinel") {
        assert_eq!(
            result
                .map(|_| String::from("passed"))
                .map_err(|error| format!("{error:?}")),
            Err(String::from(
                "RedactionViolation { code: \"redaction_violation\", secret_class: \"sentinel\", redacted_sample: \"[REDACTED:sentinel]\" }"
            ))
        );
    }
});
