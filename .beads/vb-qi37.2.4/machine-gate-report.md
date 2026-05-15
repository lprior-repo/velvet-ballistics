bead_id: vb-qi37.2.4
phase: 11
attempt: 1-of-7

STATUS: PASS

# Machine Gate Report

Commands executed from `/home/lewis/src/vb-femdation/vb-qi37-2-4` with `TMPDIR=/home/lewis/src/vb-femdation/tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1` where noted.

- `rtk cargo test --package vb_core --lib budget::vb_qi37_2_4_state8_tests -- --nocapture` => 9 passed.
- `rtk cargo test --package velvet-ballastics-workspace-tests --test vb_qi37_2_4_integration_budget_errors -- --nocapture` => 47 passed.
- `moon run :verify-standard` => PASS: All standard checks passed.
- `moon run :verify-proof` => PASS: All proof checks passed; Verus lane waived by existing gauntlet message for unavailable toolchain.
- `moon run :verify-deep` => PASS: All deep checks passed.
- `moon ci` => PASS: 20 tasks completed in 1m 6s.

Large artifact triage: untracked `budget_bounded` was an ELF verifier output and was removed with `rm -f budget_bounded`.
