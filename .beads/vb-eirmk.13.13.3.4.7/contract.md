# Together setup results

Base is 4efbab57fcec1b8da0ce00d7525816be8fcc0063, the frozen driver-fixture repair over parent ae767b77c2d054b9d5da010bc79e4c27587123d3; preserve both that ancestor and upstream proposal 529c5ec5 unchanged.

Write fence is this contract and crates/vb_runtime/src/together_tests/mod.rs plus crates/vb_runtime/src/together_tests/phase23.rs only. No production behavior, checker scripts, exclusions, suppressions, policies, timeouts, schemas, dependencies or test registration changes.

The authoritative parent r93 manifest reports 35 DISCARD-003 sites, 13 in mod.rs and 22 in phase23.rs. Each is an intended successful RunFrame write_slot or write_slot_with_taint fixture initialization, currently converted through .ok().unwrap_or_else with a failure message. Replace only those setup chains with assert_eq!(the same Result expression, Ok(()), the same message). Exact successful unit Result is required; setup failure must fail the test. An intentionally failing setup blocks the change pending actual diagnosis.

Preserve every existing behavior assertion, test name, fixture domain, input literal, helper, call ordering and taint value; compare the remaining source after removing only the reviewed setup chains and after pinned formatter normalization. No new helpers or parameter changes are needed; inherited long test functions and files are not refactored by this bounded setup repair.

Before editing, inventory both files and all Together registrations, verify exact manifest-site correspondence and run the actual ignored-fallible-results gate to capture its original failure. After editing, run pinned nightly-2026-04-28 rustfmt edition/style 2024 only on the two files, the exact existing Together unit test family with all features in vb_runtime, and the same actual ignored-result gate over the entire own candidate. Record all native cases and raw argv/stdout/stderr/exits in own evidence; no full workspace tests, Clippy, solvers or broad builds.

Use TMPDIR=/run/user/1000/vbs, own explicit target/native-vb-eirmk.13.13.3.4.7, jobs two, wrappers off and RUSTFLAGS=-Dwarnings with the existing ten-minute native timeout and 24 GiB scope. The unchanged checker is invoked with bash scripts/check-ignored-fallible-results.sh; preserve raw output including self-tests and any unrelated findings rather than waive them.

Acceptance is source-bound exact setup-success assertions plus unchanged old oracles/domains, focused native success, actual whole-candidate ignored-result gate zero, pinned file fmt zero and independent CODE/TEST. Full parent CI remains parent-owned; do not infer full acceptance or close the bead from this packet.
