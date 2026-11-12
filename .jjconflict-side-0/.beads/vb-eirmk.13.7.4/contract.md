# Contract: missing-root discovery fixture scratch ownership

Source context is candidate `da17c31407de3b40693f325b796eb01afea20704`. Change only `crates/workspace_tests/tests/vb_5xs4_test_loop_inventory_red/discovery_cases.rs::discover_returns_workspace_unreadable_when_root_marker_is_unreadable`.

Given the existing test-owned `tempfile` dependency, create a `tempfile::tempdir()?` and keep it alive for the full test; derive a missing child path beneath it with `PathBuf::join`, and never create that child.

When the test calls the existing first-party Rust-test discovery API using `WorkspaceRoot::new` for the child path, assert the exact `InventoryError::WorkspaceUnreadable` and exact root payload equal to the same child path's displayed string supplied to `WorkspaceRoot::new`.

The test may return `Result<(), Box<dyn std::error::Error>>` and propagate setup/conversion errors with `?`; preserve every other existing assertion and fixture unchanged.

Do not alter other cases, production code, public APIs, dependencies, gates, or configuration, and do not run the old literal-root scenario. This contract authorizes only the single fixture replacement; native verification and independent CODE/TEST reviews remain separate follow-up steps.
