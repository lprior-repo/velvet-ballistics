# Verus Report

bead_id: vb-qi37.16.5
updated_at: 2026-05-12T02:35:00Z
status: PASS

## Tool Availability

- `command -v verus`: `/home/lewis/.local/bin/verus`
- `verus --version`:

```text
Verus
  Version: 0.2026.05.05.d03e906
  Profile: release
  Platform: linux_x86_64
  Toolchain: 1.95.0-x86_64-unknown-linux-gnu
```
- `verusfmt --check contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs`: `VERUSFMT_MISSING`

## Contract Repair Decision

The prior exact standalone production-source commands were invalid proof targets because Verus failed before proof on crate context and Rust edition issues. State12 repaired the proof-obligation layer by adding a dedicated standalone Verus harness and updating the six Verus rows to executable harness commands. Original production files are retained as `source_target` in `proof-obligations.jsonl`.

## Exact Command Verified

```bash
verus contracts/verus/vb_qi37_16_5_lifecycle_journal_storage.rs
```

Outcome: PASS.

```text
verification results:: 12 verified, 0 errors
```

## Obligations Discharged

- `VERUS-INV-001`: `proof_single_canonical_state` proves modeled lifecycle state has one canonical enum value and is valid.
- `VERUS-PRE-002`: `proof_validate_command_precondition` proves valid lifecycle command preconditions produce accepted next state before journal write.
- `VERUS-POST-001`: `proof_append_event_injective` proves append grows journal by exactly one, preserves prior entries, and writes the requested event at the new tail.
- `VERUS-POST-003`: `proof_invalid_transition_error` proves invalid transition returns `InvalidTransition` and leaves journal unchanged.
- `VERUS-POST-004`: `proof_duplicate_request_error` proves duplicate/already-advanced command returns `DuplicateRequest` and leaves journal unchanged.
- `VERUS-POST-005`: `proof_stale_request_error` proves stale terminal-state command returns `StaleRequest` and leaves journal unchanged.

## Trust Boundary Scan

```bash
rtk grep -n 'assume\(|#\[verifier::external_body\]|#\[verifier::external\]|axiom' contracts/verus --glob '*.rs'
```

Outcome: CLEAN (`0 matches`; `TRUST_SCAN_CLEAN`).

## Trusted Boundary

The harness is a minimal mathematical model derived from `contract.md` and `verification-layers.md`: lifecycle states, lifecycle commands, command validation, and append-only `Seq<RuntimeJournalEvent>`. It excludes storage I/O, production crate dependency resolution, CLI parsing, async scheduling, and wall-clock time. No `assume`, `external_body`, `external`, or axioms were introduced.
