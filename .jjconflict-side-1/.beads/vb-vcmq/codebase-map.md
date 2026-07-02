bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 2
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

Scope map:
- Repo policy: velvet-ballistics-MASTER.md lines 147-148 list cargo-semver-checks and cargo-public-api as required public compatibility/API tools; line 171 includes cargo-public-api in install list.
- Root Cargo.toml is a pure virtual workspace. cargo-public-api upstream does not support listing an entire workspace from a virtual manifest.
- Public API evidence must therefore be per library package unless an upstream-compatible workspace wrapper is introduced.
- Touched repo files: none.
- Environment tooling touched: /cache/cargo-shared/bin/cargo-public-api installed by cargo install.
- Evidence logs: public-api-per-package.log, public-api-workspace-unsupported.log, verify-standard.log, semver-checks-workspace.log.
