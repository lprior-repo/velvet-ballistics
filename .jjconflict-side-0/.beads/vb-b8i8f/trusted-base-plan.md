# Trusted Base Plan: vb-b8i8f

## Plan Status

All trusted base entries are planned for State 5 (proof-writer) and must be reviewed at State 6 (proof-reviewer). No behavior-affecting waivers are included; all entries are proof-assumption debt that must be validated or discharged.

## Trusted Base Ledger (Planned)

| ID | Obligation | Artifact | Marker | Kind | Reason | Compensating Evidence |
|----|-----------|----------|--------|------|--------|----------------------|
| TBR-001 | PO-VERUS-001,002,003,005 | verus/cancel_kill_lattice.rs | `RunId` as opaque tracked ghost type | external_body | RunId is a newtype over u64 from vb_core; Verus models it as an opaque tracked integer. | RunId::get() and RunId::new() are trusted constructors validated by vb_core unit tests. |
| TBR-002 | PO-KANI-001,002,003 | kani_cancel_kill_lattice.rs | `Shard::new_with_journal` construction from arbitrary state | assume | Kani harnesses need Shard in a known state; production construction requires Fjall journal which is not available under kani. | Harness injects VolatileRuntimeJournal; behavior equivalence with storage journal is validated by property tests. |
| TBR-003 | PO-KANI-001,002,003 | kani_cancel_kill_lattice.rs | Journal mock for harness isolation | stub | Storage journal I/O (Fjall) is mocked in Kani harnesses to isolate pure lifecycle logic. | Property tests (PO-PROP-00x) validate journal behavior end-to-end with real storage journal. |
| TBR-004 | PO-FLUX-001,002,003,005 | flux_cancel_kill.rs | `extern_spec` for `HashMap::contains_key`, `HashSet::contains` | extern_spec | Flux needs extern specifications for standard library collections used in shard state. | Standard collections have well-defined semantics; Flux extern_specs are exact mirrors of stdlib contracts. |
| TBR-005 | PO-PROP-001,002,003 | cancel_kill_lattice_props.rs | `Arbitrary` impl for `RunId` | external_body | proptest needs Arbitrary for RunId to generate valid test values. | RunId::new() validates non-zero; proptest strategy uses new(1..=u64::MAX). |
| TBR-006 | PO-VERUS-002,003 | verus/cancel_kill_lattice.rs | `terminal_runs` as tracked ghost set | external_body | Verus models terminal_runs as ghost tracked set; production uses HashSet. | Ghost set is a conservative abstraction of HashSet membership; proved non-interference by Verus. |
| TBR-007 | PO-FLUX-002,003 | flux_cancel_kill.rs | `HashSet::contains` refinement as predicate | extern_spec | Flux refines HashSet membership to a boolean predicate for terminal state checking. | Same as TBR-004; stdlib semantics guarantee containment predicate matches production. |
| TBR-008 | PO-VERUS-004, PO-KANI-004, PO-FLUX-004, PO-FUZZ-001 | storage_kind_family.rs | `MAGIC_JOURNAL_EVENT`, `MAGIC_SNAPSHOT`, `MAGIC_BLOB` constants | assume | Magic values are compile-time constants defined in vb_storage::constants. They are trusted as correct discriminators. | Constants are verified by existing kani_record_magic.rs harness; cross-referenced with storage specification. |
| TBR-009 | PO-VERUS-004, PO-KANI-004, PO-PROP-004, PO-KANI-005, PO-PROP-005, PO-FUZZ-002 | kani_record_kind.rs | `RecordKind::id()` mapping | assume | RecordKind variant-to-u16 mapping is trusted as the authoritative wire format. | Verified by existing kani_record_kind.rs harness; enum discriminants match #[repr(u16)] annotations. |
| TBR-010 | PO-FLUX-004,005 | flux_validation.rs | Integer range refinement `10..=28` | extern_spec | Flux needs range constraint expressibility for u16 kind values in validate_kind_family. | Flux supports integer range refinements natively; range is a literal from known program text. |
| TBR-011 | PO-PROP-004, PO-PROP-005, PO-FUZZ-002 | proptest_storage.rs | postcard encode/decode stability | external_body | postcard crate is trusted for correct deterministic serialization within valid payload bounds. | postcard is a well-tested ecosystem crate; existing postcard_envelope_wire_tests validate round-trip for all existing kinds. |
| TBR-012 | PO-VERUS-005 | verus/storage_kind_family.rs | `EventSeq` as ghost-tracked u64 | external_body | EventSeq is a newtype over u64 from vb_storage; Verus models it as ghost-tracked integer with total order. | EventSeq::get() and EventSeq::new() are validated by vb_storage type_tests. |
| TBR-013 | PO-KANI-005 | kani_record_kind.rs | Fjall journal replay mock | stub | Kani cannot exercise real Fjall I/O; replay validation is tested with mocked event lists. | Property tests (PO-PROP-005) validate replay with real storage journal for end-to-end evidence. |

## Trusted Base Summary

- **Total entries**: 13
- **External bodies**: 6 (TBR-001, TBR-005, TBR-006, TBR-011, TBR-012, TBR-004)
- **Assumptions**: 4 (TBR-002, TBR-008, TBR-009, TBR-007)
- **Stubs**: 2 (TBR-003, TBR-013)
- **Extern specs**: 1 (TBR-010)
- **Behavior-affecting**: 0 (all are modeling/proof debt)
- **Review state**: planned (owner_state 5)

## Validation Plan

At State 6 (proof-reviewer), each trusted base entry must be:
1. Cited by the proof-writer in the actual proof artifacts.
2. Reviewed for soundness: does the assumption hold in production?
3. Compensated by independent evidence (property tests, existing harnesses, or explicit bridge obligations).
4. Marked `reviewer_disposition: accepted` or `rejected` with findings.

No trusted base entry waives behavior-affecting requirements.
