bead_id: vb-qi37.23
bead_title: quality: Full gate evidence refresh
phase: 11
updated_at: 2026-05-18T21:41:55Z
attempt: 3-of-7
# Machine Gate Report

STATUS: PASS
commit: e3a3f923
evidence_dir: target/vb-qi37.23-evidence/resume2-20260518T213357Z
## Gate summary

```tsv
moon-ci	0
verify-standard	0
fuzz-smoke	0
miri	0
coverage	0
mutants-smoke	0
sanitizer-address-check	0
supply-chain	0
bench-build	0
feature-powerset	0
doc-test	0
doc	0
contracts	0
source-length	0
workspace-assertions	0
public-api-workspace	2
public-api-per-package	0
bloat	0
```
## Public API waiver

- `cargo public-api --workspace --all-features` exit 2: unsupported by cargo-public-api 0.51.0 for this workspace shape.
- WVR-API-001 consumed from .beads/vb-vcmq/formal-waivers.jsonl.
- Compensating evidence: public-api-per-package exit 0 for 20 packages; raw target/vb-qi37.23-evidence/resume2-20260518T213357Z/public-api-per-package.log.
