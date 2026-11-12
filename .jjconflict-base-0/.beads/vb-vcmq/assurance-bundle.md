bead_id: vb-vcmq
bead_title: quality: provide public API evidence tooling
phase: 13
updated_at: 2026-05-18T21:14:50Z
attempt: 1-of-7

STATUS: APPROVED
Requirement evidence map:
- REQ-001 cargo-public-api installed -> PO-001 PASS -> machine-gate-report.md records cargo-public-api 0.51.0 and install command output exists in session.
- REQ-002 no fake workspace pass -> PO-002 WAIVED -> public-api-workspace-unsupported.log records exit 2 unsupported `--workspace`.
- REQ-003 compensating API evidence -> PO-003 PASS -> public-api-per-package.log records 20 library package invocations and all exit 0.
- REQ-004 unrelated verify debt classified -> PO-004 DEFERRED_GLOBAL -> verify-standard.log records known vb-ybi5 ignored-result blocker.
Waivers:
- WVR-API-001 APPROVED; compensating evidence is per-package cargo-public-api output.
No source code, test, dependency, or proof files changed.
