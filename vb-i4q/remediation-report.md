# vb-i4q Remediation Report

Bead: `vb-i4q`

Primary file remediated: `/home/lewis/src/Velvet-ballistics/velvet-ballistics-MASTER.md`

Review source: `/home/lewis/src/Velvet-ballistics/vb-7ph/doc-review.md`

## Findings Remediated

1. Hot value model carried heap-owned text in `SlotValue`.
   Changed `SlotValue` to derive `Copy`, removed the heap-owned text variant, documented text as interned `SymbolId` or bounded `BlobId` arena storage, and added a mandatory test name for text handle storage.

2. `Finish` engine contract was not mechanically compatible with non-copy slot values.
   Preserved the copy-out snippet by making `SlotValue` handle-only and `Copy`-compatible, and added an explicit note that any future non-copy value model must change the finish ownership contract first.

3. `RunFrame` constructor and taint APIs were missing.
   Added `RunFrame::new`, `read_taint`, and `write_taint` signatures and behavior, including bounds checks, admission allocation behavior, and typed error requirements.

4. Spelling exception was too broad.
   Replaced the broad exception with an exact allowlist for the current repository root path, current master filename, and explicitly labeled pre-existing migration artifacts.

5. Hot function length rule was advisory.
   Replaced the advisory line target with a hard 25-logical-line maximum for hot functions and added a required justfile/Moon/CI source-length gate.

6. Choose IR variants were ambiguous.
   Removed the generic final IR variant, kept only `ChooseExpr` and `ChooseSlot`, and defined their condition semantics and migration-only handling.

7. Action ABI referenced undefined types.
   Defined `ActionResult`, `ActionOutputReady`, `ActionFailure`, `ActionFailureCode`, `ActionError`, and expanded `ActionTicket`; added payload bounds, binary encoding, taint propagation, retry, replay, and idempotency semantics.

8. Binary record envelope was underspecified.
   Replaced the sketch with exact byte offsets, widths, endian rules, magic values, record-kind IDs, payload bounds, checksum and digest verification order, typed errors, and migration behavior.

9. MVP language weakened the final IR contract.
   Removed MVP wording from the final IR and core IR contract sections; phase sequencing now names the minimal deterministic engine primitives without weakening the final IR contract.

## Lightweight Check Scope

Checks were run against `velvet-ballistics-MASTER.md` so the remediation report does not self-match historical rejected phrases.
