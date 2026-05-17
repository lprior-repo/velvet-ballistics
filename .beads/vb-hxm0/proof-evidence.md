bead_id: vb-hxm0
phase: 5
attempt: 1-of-7

Evidence:
- rtk cargo check -p velvet-ballastics-workspace-tests: PASS, Finished dev profile.
- rtk cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog: PASS, 4 passed.
- moon run velvet-ballastics:verify-standard: FAIL due unrelated ignored fallible result violations outside touched files; classified DEFERRED_GLOBAL in regression-diff.md.
- moon ci: FAIL due unrelated fmt and vb_expr unused variable debt outside touched files; classified DEFERRED_GLOBAL.
