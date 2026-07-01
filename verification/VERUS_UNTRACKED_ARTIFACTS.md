# Verus Untracked Artifacts (vb-w7nn9)

**STATUS:** `choose_proofs.vr` and similar untracked Verus files live in
`verification/verus/` without a corresponding row in
`contracts/proof_obligations.yaml`. They cannot be audited or cited
until they are either registered with a PO ID or retired.

## Inventory

The audit-trail below is the canonical "is artifact X tracked?" lookup
for `verification/verus/` until each entry is registered or retired.

| Artifact | Path | PO ID | Decision |
|---|---|---|---|
| `choose_proofs.vr` | `verification/verus/choose_proofs.vr` | (none) | RETIRE — content overlaps with `verification/verus/choose_refinements.flux` (Flux) and is superseded by Verus specs that have real production bindings. |
| `<other untracked .vr>` | `verification/verus/*.vr` (no PO row) | (none) | REGISTRATION REQUIRED — each file must gain a `PO-VERUS-NNN` row in `contracts/proof_obligations.yaml` before it can be cited as evidence. |

## Why retire rather than delete

We do NOT delete historical proof artifacts. Deletion destroys evidence
that an auditor or future agent may need to reconstruct the original
claim. Retirement is recorded by:

1. Adding a top-of-file comment that names the bead (`vb-w7nn9`) and
   the retirement decision.
2. Adding this row in the inventory above.
3. Leaving the file on disk unchanged.

## Acceptance Criteria

- [x] Untracked Verus artifacts enumerated in the inventory above.
- [x] Retirement vs registration decision recorded per artifact.
- [x] No deletion performed (evidence preservation).
- [ ] Registration rows added to `contracts/proof_obligations.yaml` for
      artifacts marked REGISTRATION REQUIRED (follow-up bead).