# Scope
Contract vb-aqii0 addresses nine missing WorkflowParts input_slots declarations in storage admission, proptest, recovery, and durability test fixtures. These are self-contained numeric workflows with no external CLI input ports, so empty declarations are correct. No production code changes.

# Domain
Test fixtures only. The input_slots field is a required declaration even when no external inputs exist. Fixtures must declare empty input_slots to compile and validate correctly.

# Behavior
Add empty input_slots declarations to all nine affected fixtures. Preserve artifact digest construction by hashing zeroed parts after the new field. Maintain wrong digest rejection tests. Strengthen the snapshot mismatch test to assert exact snapshot_run1, sequence5, expected_run2 using the existing error variant, eliminating wildcard matching that could swallow expected_run.

# Errors
No new unwrap, panic, index, cast, or allowance constructs. Existing assertions are preserved or strengthened. The ten current compile errors in the storage library tests must resolve before any pass claim.

# Acceptance
All nine fixtures compile with empty input_slots. Digest construction and rejection tests pass unchanged. Snapshot mismatch test asserts exact run identifiers. No production changes. User workflow: contract, implementation, tests, code review, test review. No proof planning.