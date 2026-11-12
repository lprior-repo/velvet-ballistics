# 0dc ValueStore and IPC test-family port

Merge exact frozen4745ac1ecbee65939eb5e3763626f9d805ca3589 with incoming0dc854b67cee8fa87b22db8803578cf11b425ed0 in this unique colocated Git/JJ workspace. Preserve all prior changes/receipts/ancestors; no rebase/history rewrite.

Only the following test-family roots and required upstream split destinations are in scope, plus this contract.

- crates/vb_core/src/value_store/tests.rs
- crates/vb_core/src/value_store/tests/additional_coverage_taint_blob_arena.rs
- crates/vb_core/src/value_store/tests/additional_coverage_validators_default.rs
- crates/vb_core/src/value_store/tests/adversarial_bdd.rs
- crates/vb_core/src/value_store/tests/blackhat.rs
- crates/vb_core/src/value_store/tests/insert_bounds_and_accessors.rs
- crates/vb_core/src/value_store/tests/phase2_adversarial_and_arena_cap.rs
- crates/vb_core/src/value_store/tests/security_regression.rs
- crates/vb_ipc/src/tests.rs
- crates/vb_ipc/src/tests/tests_ipc_basic.rs
- crates/vb_ipc/src/tests/tests_ipc_errors_frames.rs
- crates/vb_ipc/src/tests/tests_ipc_proptests.rs

Candidate roots are byte-identical to their f08 upstream versions; preserve upstream include structure and require full expanded includes to reproduce candidate roots byte-for-byte. This is a concrete mechanical source port, not a requirement to manufacture function edits. Every existing helper, assertion, input literal, case identity, macro and proptest domain must remain unchanged; preserve all upstream new cases/code.

All13 older ordinary resolved paths remain byte-identical to1263/f08 guards; loaderfix423 already exactly matches4745 and must not be duplicated/reverted. No production typed-code changes or formal/ledger/config writes are authorized. Four inherited protected conflicts and any new try_from_parts production-mirror conflicts stay unresolved/manual proposals for their owners. Preserve every other automatically merged incoming path without manual edits.

Capture all16 incoming path hashes before/after any ordinary resolution and include-body parity/sourcecase maps. After ordinary Cargo paths are conflict-free, run at most five minutes total of focused ValueStore tests and IPC root tests default/allfeatures, own explicit target, pinned nightly2026-04-28, wrappers off, jobs2,24GiB and TMPDIR=/run/user/1000/vbs. No fullsuite, broadMoon, Kani/Verus/solver, gate or warning changes. Freeze minimal source/guards/command receipts and empty successor; partial protected snapshot is not landing/fullCI/proof acceptance.
