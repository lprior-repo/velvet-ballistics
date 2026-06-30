bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 12
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
Attack result: approve with a narrow caveat. The work fixes the original missing cargo-public-api tool. It does not fake the exact `--workspace` command; instead it records the upstream incompatibility and provides a waiver backed by per-package public API raw output. No repo source change means the verify-standard ignored-result failure is not local to this bead.
Residual risk: parent vb-qi37.23 State 11 must either consume WVR-API-001 or replace the unsupported exact command with the per-package loop. If parent blindly reruns the unsupported exact command as mandatory, it will create a new tooling-policy blocker, not a missing-tool blocker.
