# Trusted Base Plan — vb-7m21 (Replan, 14 Obligations)

## Policy

Every model bound, reduction, unavailable external reference, `assume`, `axiom`, `admit`, `external_body`, `trusted`, `ignore`, stub, disabled check, or proof-only abstraction must be ledgered by downstream states. Behavior-affecting trust is not allowed as a waiver.

## Planned Trusted Surfaces

### Kani Obligations

- **TB-vb-7m21-kani-001** for PO-vb-7m21-kani-001 (kani): Model bounds: bounded byte arrays <= RECORD_HEADER_BYTES + 4 payload cases unless explicit max-bound abstraction, use kani::Arbitrary or kani::any generators; no hardcoded structure-only proof. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-kani-002** for PO-vb-7m21-kani-002 (kani): Model bounds: bounded byte arrays <= RECORD_HEADER_BYTES + 4 payload cases unless explicit max-bound abstraction, use kani::Arbitrary or kani::any generators; no hardcoded structure-only proof. Schema version must be CURRENT_SCHEMA_VERSION + 1 with CRC recomputed. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-kani-003** for PO-vb-7m21-kani-003 (kani): Model bounds: bounded byte arrays <= RECORD_HEADER_BYTES + 4 payload cases unless explicit max-bound abstraction, use kani::Arbitrary or kani::any generators; no hardcoded structure-only proof. EOF-before-later-errors ordering preserved. No behavior-affecting trust permitted. Reviewer disposition pending.

### Proptest Obligations

- **TB-vb-7m21-prop-001** for PO-vb-7m21-prop-001 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-002** for PO-vb-7m21-prop-002 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-003** for PO-vb-7m21-prop-003 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. Truncation lengths below RECORD_HEADER_BYTES and header-only records with declared payload. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-004** for PO-vb-7m21-prop-004 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. Uses typed key constructors (run_event_key, index_status_key, index_workflow_key, index_action_key) only. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-005** for PO-vb-7m21-prop-005 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. Uses same run with seq 0 and seq 2 via events_for_run. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-006** for PO-vb-7m21-prop-006 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. Divergent duplicate event key vs identical queued duplicate scenarios. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-007** for PO-vb-7m21-prop-007 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. Snapshot sequence with newer tail events. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-prop-008** for PO-vb-7m21-prop-008 (proptest): Model bounds: deterministic seeds recorded, temporary stores only, fixture IDs unique. Uses FjallJournal::declared_keyspaces() and keyspace constants. No behavior-affecting trust permitted. Reviewer disposition pending.

### Cargo-Fuzz Obligations

- **TB-vb-7m21-fuzz-001** for PO-vb-7m21-fuzz-001 (cargo-fuzz): Model bounds: 60 second smoke plus future deep run, seed corpus generated from VB APIs/constants only. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-fuzz-002** for PO-vb-7m21-fuzz-002 (cargo-fuzz): Model bounds: 60 second smoke plus future deep run, seed corpus generated from VB APIs/constants only. No behavior-affecting trust permitted. Reviewer disposition pending.

- **TB-vb-7m21-fuzz-003** for PO-vb-7m21-fuzz-003 (cargo-fuzz): Model bounds: 60 second smoke plus future deep run, seed corpus generated from VB APIs/constants only. No behavior-affecting trust permitted. Reviewer disposition pending.

## Non-Behavior External Reference

- **TB-vb-7m21-REST-001**: External Restate record_format.rs unavailable; scope is non-behavior provenance comparison only. Compensating evidence is VB-only fixture generation review and behavior tests. This is a non-behavior-affecting external reference limitation, not a trust surface on Rust behavior.
