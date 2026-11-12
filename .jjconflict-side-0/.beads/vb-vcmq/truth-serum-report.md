bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 13
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
Execution evidence audited in active context:
- `cargo install cargo-public-api --locked` completed and installed /cache/cargo-shared/bin/cargo-public-api.
- `cargo public-api --version` returned cargo-public-api 0.51.0.
- `cargo public-api --workspace --all-features` failed with unsupported flag; this failure is disclosed and waived, not hidden.
- Per-package public API loop wrote `.beads/vb-vcmq/public-api-per-package.log`; grep check found 0 nonzero exit markers and 20 `[exit=0]` markers.
- `moon run :verify-standard` failed on vb_storage ignored fallible results; report classifies as DEFERRED_GLOBAL/known vb-ybi5, not a local pass.
Empathetic user review: parent gate should stop using unsupported cargo-public-api workspace syntax or consume the waiver to avoid reblocking operators on a confusing CLI limitation.
Skeptical QA review: evidence is honest; exact command failure is visible. Approval depends on waiver acceptance, not false command success.
