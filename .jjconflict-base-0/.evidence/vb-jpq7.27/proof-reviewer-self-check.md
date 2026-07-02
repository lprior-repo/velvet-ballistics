# Proof-reviewer self-check for vb-jpq7.27

STATUS: REJECTED_PENDING_EXTERNAL_REVIEW

Reviewer provenance: same task agent loaded `proof-reviewer` and `formal-verifier`; this is not independent approval.

Checks performed:

- PASS rows have raw log paths, commands, cwd, commit SHA, tool version, timestamp, exit code, and scope.
- PASS rows do not point at root stale summaries or prior finding files.
- Verus PASS is limited to parsing/proving the downgraded vb-jpq7.24 mirror-model artifact.
- TLC PASS rows are limited to bounded vb-jpq7.26 models that completed with exit 0.
- Kani PASS is limited to `vb_core` discovery; no harness execution is claimed.
- FAIL/BLOCKED rows have child beads: `vb-rga1`, `vb-utvm`, `vb-2tpu`.

Reason for rejection/pending status:

- This is self-review, not independent proof-review.
- `verusfmt --check` fails for the repaired Verus artifact.
- `cargo kani list` fails for `vb_validate`.
- `RecoveryReplayFull` does not complete within explicit timeout and cannot be counted as PASS.

Required external review: verify `.evidence/vb-jpq7.27/proof-obligation-ledger.jsonl` and raw logs, then approve or reject in a separate proof-review artifact.
