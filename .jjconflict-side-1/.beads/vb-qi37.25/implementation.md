bead_id: vb-qi37.25
phase: 10
STATUS: PASS

Implementation changes:
- scripts/check-workspace-assertions.py now validates exact package names, product binary names, selected feature sets, and forbidden feature names in addition to members/dependencies/generated boundaries.
- Existing workspace fixture tests now create complete manifests compatible with sharper checks.
- New vb_qi37_25_quality_gates tests assert exact failure strings and canonical spelling scan errors.
No dependency files changed. No production unsafe/panic behavior introduced.
