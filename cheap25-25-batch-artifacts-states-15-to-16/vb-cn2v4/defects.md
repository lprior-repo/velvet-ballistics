# Defects — vb-cn2v4

No defects introduced or uncovered by this verification pass.

The 3 user-mandated behavior-test commands all pass (61 + 23 + 33 = 117
tests; 0 failed). The supplementary full-suite and check commands also
pass (1674 + 69 = 1743 additional tests; 0 failed; workspace compiles
clean). The State 11 holzman-rust commit `xrpxwkvz a47b72c6` is the
working-copy baseline; all touched files (`crates/vb_storage/src/keys.rs`,
`crates/vb_storage/src/keys/tests.rs`, `crates/vb_storage/src/kani_typed_partitioned_ids.rs`,
`crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs`,
`crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs`,
`crates/workspace_tests/tests/vb_eepg_bdd_tests.rs`) compile under
`cargo check --workspace --all-targets --all-features` and the
production-target clippy (`-p vb_storage --lib --bins --all-features`
with all -D flags) is green.

Pre-existing global debt (vb_core red test, repo-wide fmt drift) is
documented in `formal-verification-report.md` and `black-hat-review.md`
as out of scope per contract C9. They are not defects of this bead.
