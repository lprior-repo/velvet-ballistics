# Workspace assertion fixture driver retention

Parent source is ae767b77c2d054b9d5da010bc79e4c27587123d3; implementation is a new isolated JJ child, preserving frozen upstream proposal 529c5ec5fc17c6d99d8074eddb683bbd6bf27f32.

Write fence is this contract and crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs plus crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs; no checker, driver, gate, manifest, dependency, policy, timeout or schema changes are authorized.

The checker root declares mod check_workspace_assertions_driver at scripts/check-workspace-assertions.rs:253; every synthetic workspace that copies the root must also copy the actual scripts/check_workspace_assertions_driver.rs bytes beside it before invoking the existing shell entry point.

Caller inventory covers exact checker root/shell/driver references throughout crates and scripts: the two fixture builders above are the only synthetic script copiers; vb_8ma2 routes both workspace_with and workspace_with_dependency_line through copy_assertion_scripts, while vb_qi37_25 routes its three tests through workspace. Preserve all seven plus three registered names, every original test body and assertion, all manifest fixture inputs and expected diagnostics.

Add the missing driver copy once per fixture builder, propagate each fs::copy failure with Result/?, and keep modified or extracted helpers at most 25 logical lines including signature and at most five parameters. A small private script-copy helper may be extracted within the second target to keep workspace bounded; no shared framework or new cross-target module is required.

Before repair, copy the unchanged root and shell into an owned evidence fixture and compile through the existing pinned rustc route: it must fail with E0583 naming check_workspace_assertions_driver. Restore the actual driver bytes to that same fixture and repeat compilation: it must exit zero. Keep all control files and raw argv/stdout/stderr/exits under this workspace target/core-build-20261110/evidence.

After repair, pinned nightly-2026-04-28 rustfmt edition/style 2024 checks the two fenced test files; the original two nextest targets must each execute in default and all-features lanes, with RUSTFLAGS=-Dwarnings, wrappers disabled, jobs two, an explicit own target/native-vb-eirmk.13.13.3.4.5 and a 24 GiB memory scope. Record actual native exits and per-target counts, not a client or tail-pipeline exit.

Acceptance requires an exact source-bound patch, full original test-body preservation hashes, unchanged checker/root/shell hashes, bounded helper ledger, raw commands and exit receipts, then independent CODE and TEST review. Focused results do not establish full workspace or canonical CI acceptance; parent owns the combined native run and no bead closure is inferred here.
