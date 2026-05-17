# vb-jkrk baseline report

Workspace: `/home/lewis/src/Velvet-ballistics-vb-jkrk-go`

## Known release blockers from vb-qi37.16.3

Source: `/home/lewis/src/Velvet-ballistics-vb-qi37-16-3-go/.beads/vb-qi37.16.3/landing-blocker.md`

- `moon ci`: FAIL
- `velvet-ballastics:fmt`: formatting diffs in proof kernels, storage, fuzz, and xtask files
- `velvet-ballastics:lint-src`: `vb_proof_kernels::EnvelopeHeader::new` lacks `Default`; `xtask/src/proof.rs` panic via `unwrap_or_else(|| panic!(...))`
- `velvet-ballastics:feature-powerset`: `vb_ui_model --no-default-features` fails because `Vec` is unavailable and module-level `#![cfg_attr(not(feature = "std"), no_std)]` attributes are invalid outside crate root

## Local setup observations

- The pre-existing suggested path was a copied JJ repo and caused trybuild path mismatches against `/home/lewis/src/Velvet-ballistics/...`; that was an environment/setup artifact, not an acceptance blocker.
- Recreated the path as a real JJ workspace before continuing CI repair.

## Baseline commands

- `moon ci` in stale copied repo: FAIL after acceptance blockers (`fmt`, `lint-src`, `feature-powerset`) had passed; failure was `velvet-ballastics:test` trybuild stderr path mismatch caused by copied repo path leakage.
- Recreated real workspace with `jj workspace add --revision main --name vb-jkrk-go ...`: PASS.
- `jj status` in recreated workspace: PASS/no changes.

## Real JJ workspace reproduction and outcome

- `rtk cargo fmt --check`: PASS.
- `moon run :lint-src`: PASS, `1 completed`, `Time: 1s 244ms` after repair.
- `moon run :feature-powerset`: PASS, `4 completed`, `Time: 1m 26s 35ms`; included `vb_ui_model --no-default-features` and `--features std/default` lanes.
- `moon run :fmt`: PASS, `1 completed`, `Time: 2s 525ms`.
- `moon ci`: PASS, `Tasks: 19 completed (1 cached)`, `Time: 2m 50s 212ms`.

Acceptance blockers are resolved or already green on current `main` baseline:

- `fmt`: PASS.
- `lint-src`: PASS after removing `xtask/src/proof.rs` panic path; `EnvelopeHeader` already has `Default` on this baseline.
- `feature-powerset`: PASS; `vb_ui_model --no-default-features` passes on this baseline.
