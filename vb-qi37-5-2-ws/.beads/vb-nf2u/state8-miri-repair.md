STATUS: FAIL

Root cause: the recorded `getcwd` failure came from proptest file failure persistence under Miri isolation in `gate_08_accessor` proptests. Miri does not support `std::env::current_dir` under isolation, and proptest's default file persistence path calls it while materializing failure persistence.

Files changed:
- `crates/vb_validate/src/gate_08_accessor.rs`

Commands run:
- `moon run velvet-ballastics:fmt` — PASS (`Tasks: 1 completed`).
- `cargo +nightly-2026-04-28 miri test -p vb_validate --lib gate_08_accessor::tests::proptest_above_bound_field_fixtures_use_checked_construction` — PASS (1 passed, 907 filtered).
- `moon run velvet-ballastics:miri` — FAIL. The original proptest failure-persistence/getcwd issue did not recur; Miri progressed and failed later in `vb_validate` with assertion failures in `gate_08_accessor::tests::proptest_gate_08_reports_first_invalid_accessor_with_root_precedence` and `gates::tests::gate_08_accepts_valid_accessor`. Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e0fd5ca21001HsuZp4cMN4EGFv`.

Normal proptest coverage preservation: the repair only sets `failure_persistence: None` for this existing `proptest!` block. It does not skip or reduce generated cases, ranges, assertions, or normal non-Miri execution; it only disables writing persisted failure files.

Residual next failure/full CI status: full `moon ci --base HEAD --head HEAD` was not run. The next observed Miri failure appears to be a behavior/test expectation failure in gate 08 validation after the Miri persistence issue was removed; it needs separate classification before repair.
