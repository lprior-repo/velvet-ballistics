# Trusted Base Plan — vb-t0iw9

schema_version: trusted-base-plan/v1
state: 4
bead_id: vb-t0iw9

This file records every trust marker an obligation raises so the
formal-verifier can carry a `trusted-base-ledger/v1` row at State 12. Each
obligation's `trusted_base_refs` lists the IDs it consumes; the bodies
below describe the corresponding assumption boundary.

## TB-T0IW9-bd-stderr-grammar

- assumption: bd stderr strings are bounded at 4096 bytes per error emission.
- source: codebase-map.md §34-41 (live schema-error reproduction table).
- boundary: enforces `cargo fuzz … -max_len=4096` for the
  `SchemaErrorClass_parse_fuzz.rs` target; PO-T0IW9-002 bound.
- verifier responsibility: run a sanity smoke `bd supersede vb-qryp7 --with vb-t0iw9 2>&1 | wc -c`
  against the post-repair `.beads/` and persist the byte count under
  `.beads/vb-t0iw9/post-repair-verification/bd-stderr-byte-count.md`.
- risk of being wrong: a bd stderr emission over 4096 bytes would not be
  exercised by the fuzz corpus; the bound must be re-justified if a
  longer emission is observed in the post-repair smoke.

## TB-T0IW9-beads-config-precedence

- assumption: BEADS_DOLT_* > metadata.json > config.yaml precedence is
  authoritative; repairs to `dolt.server-port`/`dolt.server-host` go into
  `BeadsConfig`; repairs to `dolt_database`/`project_id` go into
  `BeadsMetadata`; cross-mixing is forbidden.
- source: AGENTS.md lines 21-39 (Beads Dolt Remote), contract.md:OB-007,
  type-contracts.md § ConfigKey/MetadataKey.
- boundary: enforces `cargo fuzz … -max_len=4096` for
  `BeadsConfig_BeadsMetadata_fuzz.rs`; PO-T0IW9-003 bound.
- verifier responsibility: after fuzz exhaustion, inspect the rejected
  corpus entries and confirm each maps to one of the documented
  precedence-inversion / cross-mixing / illegal-config-key patterns.
- risk of being wrong: an undocumented precedence rule could allow a
  fuzz entry that the corpus rejects but a real bd binary accepts; the
  precedent is the live `bd v1.0.5` source (out of scope for this bead,
  pinned via the `bd version 1.0.5 (dev)` capture in OB-001).

## TB-T0IW9-depends-on-id-stored-generation

- assumption: `dependencies.depends_on_id` is a STORED generated column
  as of `bd v1.0.5` per migrations 0041-0042; the migration chain is
  intentionally irreversible; re-adding the column as plain breaks the
  contract.
- source: contract.md:OB-006, hazard-analysis.md HAZ-008,
  type-contracts.md § Repair decision table (GenerationColumnDrift
  default legal decision is `DocumentExpectedUserAction` if STORED,
  `Escalate` if not).
- boundary: enforces `cargo fuzz … -max_len=4096` for
  `AddSchemaMigration_statement_fuzz.rs`; PO-T0IW9-004 bound.
- verifier responsibility: capture `bd info --whats-new | sed -n
  '/0041-0042/p'` under
  `.beads/vb-t0iw9/post-repair-verification/bd-whats-new.md` and confirm
  that the migration text in the captured record still names the
  STORED-generation contract for `depends_on_id`.
- risk of being wrong: a future `bd` binary that changes the
  STORED-generation contract would invalidate this trust marker; the
  fingerprint is captured at OB-001 (`bd version 1.0.5 (dev)`) and is
  the migration-chain trust root for this bead.

## TB-T0IW9-bd-server-stable

- assumption: the live shared Dolt server at 127.0.0.1:45645 (database
  `velvet-ballistics`) is reachable from the isolated workspace and
  supports the documented verification command surface during the
  bead's lifecycle.
- source: codebase-map.md §26, contract.md:OB-009.
- boundary: enforces `bash scripts/check-beads-server-mode.sh` plus the
  six verification CLIs (`bd dolt status`, `bd dolt test`, `bd info`,
  `bd migrate`, `bd sql "SHOW COLUMNS FROM dependencies"`,
  `bd supersede vb-qryp7 --with vb-t0iw9`); PO-T0IW9-005 bound.
- verifier responsibility: capture `bd dolt status` output at
  `.beads/vb-t0iw9/post-repair-verification/bd-dolt-status.md` and
  pin the recorded `host:port` against the captured `127.0.0.1:45645`.
- risk of being wrong: a Dolt server restart on a different port (or a
  network partition during the post-repair smoke) would invalidate the
  proptest property-pressure run; this is exactly the failure the
  anti-invariant in PO-T0IW9-005 catches, so the obligation is robust.

## OB-specific trust markers that did not need a TB-NNN

The following obligations emit no trust marker because their assumption
list is empty:

- PO-T0IW9-001 (BdVersion byte-stability across probes): no
  assumptions; assumes only a properly-resolved `bd` binary on `PATH`.
  Verification is closed by a single `bd version` round-trip; the
  assumption is not a trust marker and does not require a TB-NNN row.
- PO-T0IW9-002 (SchemaErrorClass::parse hostile-input): only the bd
  stderr byte-count assumption above; addressed by
  TB-T0IW9-bd-stderr-grammar.
- PO-T0IW9-003 (BeadsConfig/BeadsMetadata round-trip): only the
  BEADS_DOLT_* precedence assumption; addressed by
  TB-T0IW9-beads-config-precedence.
- PO-T0IW9-004 (AddSchemaMigration::statement parser): only the
  STORED-generation assumption; addressed by
  TB-T0IW9-depends-on-id-stored-generation.
- PO-T0IW9-005 (post-repair verification): only the live server
  reachability assumption; addressed by TB-T0IW9-bd-server-stable.

## Out-of-trust markers (acknowledged, not raised)

The following are tracked in `verifier-lane-matrix.md §4` as
`not_applicable` and have no TB-NNN row:

- `verus`, `kani`, `flux-rs`, `loom`, `miri`: their absence is a
  `limitation_kind: surface_absent` (no production Rust) or
  `limitation_kind: risk_out_of_scope` (no concurrency). These are
  reviewer-owned disposition items at State 4b and are not trust markers
  because there is no `exec fn`/`#[kani::proof]`/`#![flux::cfg]` to
  lean on.
- `tla-plus` (legacy, removed): skill policy.
