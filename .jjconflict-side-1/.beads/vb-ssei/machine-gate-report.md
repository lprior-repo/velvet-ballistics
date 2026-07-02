STATUS: PASS

# Machine gate report

Scoped PASS gates:
- `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_ssei_verification_admission_acceptance -- --nocapture` -> `cargo test: 4 passed (1 suite, 0.00s)`.
- `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog -- --nocapture` -> `cargo test: 6 passed (1 suite, 0.00s)`.
- `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_ssei_verification_admission_acceptance --test vb_hxm0_acceptance_catalog -- --nocapture` -> `cargo test: 10 passed (2 suites, 0.02s)`.
- `rtk cargo fmt -p velvet-ballistics-workspace-tests -- --check` -> PASS, no output.
- `rtk cargo check -p velvet-ballistics-workspace-tests` -> `cargo build (21 crates compiled)` and finished successfully.

Canonical `moon ci` observed but failed only on unrelated existing global debt; see `regression-diff.md`.
