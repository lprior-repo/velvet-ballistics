bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Verification Layers

## Layer Assignment

### Preconditions (P1-P3)
- **Layer**: Unit tests + OS integration tests
- **Tool**: `cargo test -p vb_storage`
- **Rationale**: Filesystem and flock behavior verified by actual I/O

### Postconditions (PO1-PO6)
- **Layer**: Unit tests + Integration tests
- **Tool**: `cargo test -p vb_storage`, `cargo test -p velvet_ballastics`
- **Rationale**: Direct observable behavior on real filesystem

### Invariants (I1-I4)
- **Layer**: Unit tests + Adversarial tests
- **Tool**: `cargo test -p vb_storage security_tests`
- **Rationale**: Security properties require adversarial verification

## Defense-in-Depth Summary

| Concern | Primary Verification | Compensating Control |
|---|---|---|
| Dual writer prevention | Unit test: second open fails | Security test: exact error type |
| Lock release | Unit test: drop then re-open | Rust Drop semantics |
| Doctor reporting | Integration test: doctor fails on locked DB | CLI exit code verification |
| No mutation before lock | Code review: lock before keyspaces | Security test |
