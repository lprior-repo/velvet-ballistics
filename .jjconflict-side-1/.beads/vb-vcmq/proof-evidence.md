bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 5
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
Evidence:
- cargo install cargo-public-api --locked: PASS, installed cargo-public-api v0.51.0.
- cargo public-api --version: PASS, cargo-public-api 0.51.0.
- cargo public-api --workspace --all-features: exit 2, unsupported upstream flag; see public-api-workspace-unsupported.log.
- per-package cargo public-api: PASS all 20 packages; see public-api-per-package.log and rtk grep summary (0 nonzero markers, 20 exit=0 markers).
