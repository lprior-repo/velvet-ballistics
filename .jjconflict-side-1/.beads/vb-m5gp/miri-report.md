# Miri Report

STATUS: DEFERRED_GLOBAL

- Obligation: `MIRI-001` is `required:false`.
- Command: `cargo +nightly miri test -p vb_compile`.
- Result: DEFERRED_GLOBAL — direct command failed before tests because `/home/lewis/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library` does not exist.
- Compensating evidence: canonical `moon ci` Miri lane passed selected checks.
- Follow-up: repair local nightly rust-src/Miri toolchain path and rerun direct `cargo +nightly miri test -p vb_compile`.
