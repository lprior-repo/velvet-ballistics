# vb-jpq7 closure evidence manifest

`scripts/check-vb-jpq7-closure-evidence.py` fails closed unless every closed
`vb-jpq7.*` child has a JSONL row in
`.beads/vb-jpq7/closure-evidence-manifest.jsonl` with raw command evidence.

Required row fields: `bead_id`, `command`, absolute `cwd`, `commit_sha`, `tool_version`,
UTC-`Z` `timestamp`, `raw_log_path`, `stdout_summary`, `stderr_summary`,
`exit_code`, `status`, and `evidence_kind`.
Rejected evidence shapes include summary-only, cached-only, skipped-only,
subagent-only, and delegated-only rows.

The canonical Moon `blocker-closure-evidence` task runs in CI against live bd
children from `VB_JPQ7_BD_WORKDIR`, falling back to
`/home/lewis/src/velvet-ballistics`. Fixture data is only used inside
`--self-test`; it is not exposed as a CI acceptance path. Raw logs must exist,
be non-empty, and contain
manifest-bound provenance lines for `command`, `cwd`, `timestamp`, and
`exit_code`; historical `cwd` directories need not still exist when the raw log
binds the original working directory. Missing-log bypasses are not available in
gate mode.

Non-zero rows are not passing closure evidence. They must declare
`resolution_kind` as `split_followup` or `approved_waiver`, include a non-empty
`resolution_rationale`, and point to the corresponding `split_bead_id` or
`waiver_id`. Reusing one split bead requires a distinct rationale per row.

```json
{"bead_id":"vb-jpq7.14","command":"moon ci","cwd":"/home/lewis/src/velvet-ballistics","commit_sha":"abcdef1","tool_version":"moon 1.35.5; cargo 1.91.0-nightly","timestamp":"2026-05-23T00:00:00Z","raw_log_path":"/home/lewis/.local/share/opencode/tool-output/tool_example","stdout_summary":"Tasks completed; tests passed","stderr_summary":"empty","exit_code":0,"status":"PASS","evidence_kind":"raw-command"}
```

Useful commands:

```bash
python scripts/check-vb-jpq7-closure-evidence.py --self-test
python scripts/check-vb-jpq7-closure-evidence.py --parent vb-jpq7 --bd-workdir /home/lewis/src/velvet-ballistics
```
