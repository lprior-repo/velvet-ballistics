Source-length gate behavior-test fixtures.

The State 9 tests construct temporary git repositories at runtime so the
gate can exercise `git ls-files`. Static fixture files in this directory use
the `.rs.fixture` suffix instead of `.rs` so the repository's own source-length
gate does not count intentionally over-budget fixture files as production Rust.
