bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 3
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
Layer 1 tool availability: `rustup run nightly-2026-04-28 cargo public-api --version` -> cargo-public-api 0.51.0.
Layer 2 exact-command honesty: `.beads/vb-vcmq/public-api-workspace-unsupported.log` records upstream `--workspace` rejection; no pass claimed.
Layer 3 compensating API evidence: `.beads/vb-vcmq/public-api-per-package.log` has 20 package commands, all `[exit=0]`.
Layer 4 canonical gate classification: `.beads/vb-vcmq/verify-standard.log` records unrelated vb_storage ignored-result blocker already tracked by vb-ybi5.
