# vb-odhel source-length rebaseline evidence

## Finding

The pre-push `moon ci` run in
`/home/lewis/.local/share/opencode/tool-output/tool_e7432c31f002anHbFi3WnYXEle`
failed `velvet-ballistics:source-length` because
`crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs` has 668
physical lines and did not have a valid `.config/source-length-exceptions.txt`
row.

## Repair

Added the missing source-length exception row, tied to the existing split owner
`vb-jpq7.47`, with an explicit release-before-split note for the Restate
postcard newtype compatibility test family.

## Verification

```text
$ bash scripts/check-source-length.sh
<no output; exit 0>

$ moon run velvet-ballistics:source-length
Tasks: 6 completed (1 cached)
Time: 57s 904ms

$ moon ci
Raw output: /home/lewis/.local/share/opencode/tool-output/tool_e745440b4001j0vVR1JunnsI4R
Tasks: 32 completed (5 cached)
Time: 9m 40s 645ms
```

## Residual risk

This is a rebaseline only. The file remains oversized and must be split by the
`vb-jpq7.47` release-before-split owner before the exception can be removed.
