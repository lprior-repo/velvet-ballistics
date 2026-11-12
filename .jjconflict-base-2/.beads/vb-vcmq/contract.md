bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 3
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
REQ-001: The cargo-public-api subcommand SHALL be installed and callable under nightly-2026-04-28.
REQ-002: The parent exact command `cargo public-api --workspace --all-features` SHALL NOT be faked as passing if upstream rejects `--workspace`.
REQ-003: If the exact workspace invocation is unsupported, a required-tool waiver SHALL include raw compensating evidence for every library package in the workspace.
REQ-004: Existing unrelated verify-standard debt SHALL be classified separately and not hidden.
Postcondition: vb-qi37.23 can rerun State 11 with either cargo-public-api present or consume the waiver evidence for public API compatibility.
