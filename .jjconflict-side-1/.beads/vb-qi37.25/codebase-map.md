bead_id: vb-qi37.25
bead_title: quality: Workspace assertion sharpness and spelling gates
phase: 2
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

Mapped scope:
- scripts/check-workspace-assertions.py: workspace membership, package/bin/feature/dependency/generated-boundary assertions.
- scripts/check-workspace-assertions.sh: shell entrypoint for moon :workspace-assertions.
- crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs: existing workspace assertion tests updated for sharper fixture manifests.
- crates/workspace_tests/tests/vb_qi37_25_quality_gates.rs: new mutation-resistant exact assertion/spelling tests.
- crates/vb_cli/src/naming_scan/*: existing canonical spelling scan API exercised, no production changes needed.
