# Coordinator evidence audit

Scope: codec-only changes relative to unaccepted prototype5fff342a. No authorization/replay/grant acceptance.

Fresh source-frozen coordinator runs (official runner, real pinned nightly):
- coordinator-frozen-tests-v1: cargo test -p vb_storage --test policy_decision_wire, exit0, 7 passed, no ignored/filtered.
- coordinator-frozen-strict-lint-v1: cargo clippy -p vb_storage --lib -- -D warnings, exit0.
- coordinator-frozen-fmt-v1: cargo fmt -p vb_storage --check, exit0.

Worker handoff corrections: V4-FRESH-2 failed because the worker omitted Cargo separator -- before -D warnings. The supplied gate runner passes argv unchanged; it does not strip -D. V4-FRESH-2B was weaker clippy with no -D and is not strict acceptance. V4-FRESH-3 deliberately chose wrong +1.89.0 and whole workspace; failed1 on unrelated prototype formatting. It is not a pinned formatter pass. These failures remain preserved. Fresh coordinator commands above supersede these for this scoped candidate.

New malformed-payload test now uses checked_sub and safe get; no new panic/unwrap/expect/index/as/allow found in its source. No preexisting tests/assertions deleted in the final net slice. Storage tests currently preserve old incomplete policy event shape: full ticket and pinned admission identity still need schema adaptation from the separate foundation. Independent code and test reviews remain required.
