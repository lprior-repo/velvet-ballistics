bead_id: vb-hxm0
phase: 11
attempt: 1-of-7

Classification: DEFERRED_GLOBAL

Touched files are limited to crates/workspace_tests/src/lib.rs, crates/workspace_tests/src/acceptance_catalog.rs, and crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs.

Global failures observed:
- FORMAT: rustfmt wants unrelated benches/tests/fuzz files changed.
- CLIPPY/CHECK: crates/vb_expr/src/eval.rs unused variables at lines 560, 568, 576, 584, 592, 600.
- GATE-IGNORED-FALLIBLE-RESULTS: unrelated files listed in machine-gate-report.md.

No observed failure is in delivery scope. Follow-up beads already visible in ready list include vb-ib8i and vb-n746 for canonical moon ci/global failures.
