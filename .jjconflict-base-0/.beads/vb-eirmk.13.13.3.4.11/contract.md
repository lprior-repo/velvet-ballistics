# Kani decode child module loading

Base is frozen39c94920fee502fd223d9d81aaba61d9dece3fe2 in this unique colocated Git/JJ workspace. Preserve all ancestors, four protected conflicts and every other source byte.

Write fence is this contract and crates/vb_ipc/src/verification/kani/kani_flag_validation.rs only. Add #[path = "kani_flag_validation/decode_integration.rs"] immediately before its sole mod decode_integration declaration; preserve every harness, domain, assertion, bound, child byte and registration name. The real crate caller in lib.rs is cfg(kani) with #[path = "verification/kani/kani_flag_validation.rs"]. No gate/config/dependency/waiver changes.

Baseline parentSHA904f11b2dd2e9b6c252f266d9fa89e7fea4f7654c3ee88bc9572a7e1666f0cf4; exact childSHA3377d915a9956f9a77b2d0d4366d443682a460cdf0bf75583dbd974875de37a6. Record actual complete caller/declaration inventory and compare parent bytes by exact one-line insertion; keep child and lib.rs hashes unchanged.

Capture the pinned nightly-2026-04-28 canonical cargo fmt --all -- --check failure and targeted rustfmt module-load before/after receipts; the missing-child loader error must disappear. Existing unrelated canonical formatting gaps or protected proposals stay unapplied; do not reformat harness source. Read-only rustfmt --emit stdout may save formatted copies under owned ignored evidence.

Compiler-resolution check is one supported cargo kani --only-codegen -p vb_ipc --lib --all-features --harness decode_roundtrip_valid_flags -j1 with CARGO_HOME=/cache/cargo-shared selecting installed Kani0.68, wrappers off, own explicit target, jobs1, 24GiB memory and five-minute timeout. No solver, heavy campaign, retries or cap increases. Report unrelated codegen failure exactly without source expansion. Tooling/module-loading health is not proof acceptance.

Use TMPDIR=/run/user/1000/vbs for every command; all raw argv/log/status/exit/hash receipts are owned target/core-build-20261110/evidence/kani-module-loader. Freeze source plus empty successor and exact parent/contract bindings for independent CODE/TEST, without claiming current full formatter, native or Kani proof acceptance.
