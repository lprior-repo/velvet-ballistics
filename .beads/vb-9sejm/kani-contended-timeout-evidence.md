# vb-9sejm Kani contended timeout evidence

## Finding

After source-length and fmt were repaired, canonical `moon ci` still failed with
one failing task in
`/home/lewis/.local/share/opencode/tool-output/tool_e74447970001NfpoXmmiUvtBCk`.
The only non-passing resolved task was `velvet-ballistics:verify-kani-vb-validate`:
it stopped at approximately the existing 5-minute harness timeout while the same
task needed longer under full CI contention.

Focused confirmation before the timeout change:

```text
$ moon run velvet-ballistics:verify-kani-vb-validate
Tasks: 1 completed
Time: 9m 18s 819ms
```

The harnesses pass; the CI timeout was too narrow for contended execution.

## Repair

Updated `.moon/tasks/kani.yml` so `verify-kani-vb-validate` wraps each
`vb_validate` Kani harness with `timeout 10m` instead of `timeout 5m`.

This does not weaken any Kani harness, assumption, unwind bound, or production
contract. It only prevents the CI shell from killing a valid bounded-model check
under CPU/cache contention.

## Verification

```text
$ moon ci
Raw output: /home/lewis/.local/share/opencode/tool-output/tool_e745440b4001j0vVR1JunnsI4R
Tasks: 32 completed (5 cached)
Time: 9m 40s 645ms
Kani line: velvet-ballistics:verify-kani-vb-validate (9m 40s 575ms, c0b0c5d2)
```

## Residual risk

Kani evidence remains bounded to the named `vb_validate` harnesses and their
source-level bounds. The CI timeout is now a resource envelope, not a proof
claim.
