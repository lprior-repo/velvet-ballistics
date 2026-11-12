bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 7
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
Tests/gates:
1. Tool availability: `cargo public-api --version`.
2. Exact parent command honesty: capture `cargo public-api --workspace --all-features` failure.
3. Compensating evidence: run `cargo public-api -p <package> --all-features` for every library package from cargo metadata.
4. Canonical verify classification: run `moon run :verify-standard` and classify unrelated failure.
No Red Queen. No test files added because behavior is external tooling availability.
