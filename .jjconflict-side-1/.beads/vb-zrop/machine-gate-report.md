bead_id: vb-zrop
phase: 11

STATUS: PASS

Commands:
- `bash scripts/check-ignored-fallible-results.sh` -> exit 0; evidence `.beads/vb-zrop/focused-ignored-results.log`; output includes `NoViolationFound`.
- `moon run :verify-standard` attempt 1 -> exit 1; ignored-result gate passed but KANI-ACCESSOR-REF-001b/001c failed on non-exhaustive PathSegment match; evidence `.beads/vb-zrop/verify-standard.log`.
- `moon run :verify-standard` attempt 2 -> exit 0; evidence `.beads/vb-zrop/verify-standard-2.log`; output includes `All standard checks passed`.
- `moon ci` -> exit 0; evidence `.beads/vb-zrop/moon-ci.log` and `.beads/vb-zrop/moon-ci.exit`.
