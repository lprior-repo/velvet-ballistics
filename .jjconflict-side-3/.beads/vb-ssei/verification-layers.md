bead_id: vb-ssei
phase: 3
updated_at: 2026-05-18T21:50:13Z
attempt: 1-of-7

# Verification layers

- Layer 1: BDD acceptance tests in `vb_ssei_verification_admission_acceptance.rs`.
- Layer 2: Catalog traceability test in `vb_hxm0_acceptance_catalog.rs`.
- Layer 3: Package compile/type check: `rtk cargo check -p velvet-ballistics-workspace-tests`.
- Layer 4: Touched package format check: `rtk cargo fmt -p velvet-ballistics-workspace-tests -- --check`.
- Layer 5: Canonical moon CI observed; unrelated global failures classified `DEFERRED_GLOBAL`.
