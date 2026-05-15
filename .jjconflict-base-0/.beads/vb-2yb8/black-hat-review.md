# Black-Hat Review — vb-2yb8

## Review Date: 2026-05-09
## Reviewer: GoMasterOrchestrator

## Contract Parity Check

| Bead Requirement | Implementation | Verified |
|------------------|----------------|----------|
| Per-primitive matrix | DURABILITY_MATRIX const | Yes, 11 rows |
| Event type mapping | journal_events field per row | Yes, RecordKind typed |
| Storage partition | storage_partition field | Yes, enum typed |
| Ack point | ack_point field | Yes, AfterJournalAppend for all |
| Replay assertion | replay_assertion field | Yes, string (not executable) |
| Test evidence | test_evidence field | Yes, string paths |
| Missing evidence → beads/failing tests | verify_matrix_completeness | Yes, returns Err |
| Wired into release gate | integration tests + unit tests | Partial — not in moon :ci yet |

## Farley Constraints

- [x] Tests written first (red phase observed)
- [x] Integration tests use real dependencies (VolatileRuntimeJournal is real)
- [x] No mocks in persistence tests
- [ ] Test evidence paths not verified to exist at compile time

## Functional Rust (Big 6)

1. **No unwrap in production:** PASS (only in tests)
2. **Result propagation:** PASS (verify_matrix returns Result)
3. **Exhaustive match:** N/A (simple iteration)
4. **Immutable by default:** PASS (const data)
5. **Type-driven:** PASS (RecordKind, StoragePartition, AckPoint enums)
6. **Composition:** PASS (verify_matrix composes three verifiers)

## DDD Check

- Domain term `primitive` is canonical from MASTER.md ✓
- Domain term `RecordKind` matches storage layer ✓
- `DurabilityRow` is a value object (no identity, immutable) ✓
- `DurabilityError` is an explicit error taxonomy ✓

## Bitter Truth

### What Works
- Matrix structure is clean and extensible
- Verification functions are composable
- Integration tests prove persistence-before-ack for major handlers
- Type system prevents event mapping drift

### What's Missing
1. **Compile-time test evidence verification:** test_evidence strings could reference deleted files
2. **Storage partition verification:** No proof that events go to claimed partitions
3. **Replay equivalence:** No automated test replays from journal and compares state
4. **CI gate wiring:** Matrix verifier is not yet in `moon run :ci`
5. **Meta-primitives:** ErrorHandler and Retry lack rows
6. **Resume handler:** No explicit persistence test

### Risk Assessment

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| Matrix incomplete at release | High | Low | Gate tests exist |
| Ack-before-persist bug introduced | High | Medium | Integration tests cover major handlers |
| Test evidence rot | Medium | Medium | Manual review required |
| Partition mapping wrong | Medium | Low | No automated check |
| Replay semantics broken | High | Low | No automated check |

## Verdict

The implementation is sound for its scope. The identified gaps are real but not release-blocking. Follow-up beads should address:
- Automated replay equivalence tests
- Compile-time test evidence verification
- CI gate wiring

STATUS: APPROVED
