# Workflow Model — vb-om21

## Workflow: Tail Scan Fallback

### States

1. `Requested { run, mode, metadata }`
2. `PrefixBuilt { run, prefix }`
3. `Scanning { run, prefix, latest? }`
4. `ObservedEmpty { run }`
5. `ObservedPresent { run, max_seq, reconstructed_tail }`
6. `MetadataAccepted { run, tail }`
7. `RecoveredWithTail { run, tail }`
8. `Failed { error }`

### Transitions

| From | Guard | To | Outcome |
|---|---|---|---|
| `Requested` | `run_prefix_key(run)` succeeds | `PrefixBuilt` | prefix fixed to `[0x11][run_id_be]` |
| `PrefixBuilt` | snapshot/range/prefix scan starts | `Scanning` | scan is bounded to prefix |
| `Scanning` | no key with prefix | `ObservedEmpty` | reconstructed tail is zero for query mode |
| `Scanning` | key starts with prefix and has sequence bytes | `Scanning` | latest = max(latest, decoded seq) |
| `Scanning` | first non-prefix key reached | `ObservedPresent` or `ObservedEmpty` | terminate scan; do not cross run boundary |
| `ObservedEmpty` | mode is `QueryAllowsEmpty` | `RecoveredWithTail` | tail = 0 |
| `ObservedEmpty` | mode is `RecoveryRequiresJournal` | `Failed` | `MissingJournal { run }` |
| `ObservedPresent` | metadata missing | `RecoveredWithTail` | fallback to reconstructed tail |
| `ObservedPresent` | metadata present and `declared >= reconstructed` | `MetadataAccepted` | metadata is not below durable key tail |
| `ObservedPresent` | metadata present and `declared < reconstructed` | `Failed` | `TailMismatch` |
| `MetadataAccepted` | recovery requested | `RecoveredWithTail` | recover without warning |
| any | Fjall/key/decode/overflow error | `Failed` | typed error; no silent continuation |

## Terminal Outcomes

- Success: `RecoveredWithTail { run, tail }`.
- Typed failure: `TailMismatch`, `MissingJournal`, storage/key parse failure, or tail overflow.

## Temporal Properties

1. Scan termination is guaranteed by Fjall iterator exhaustion or first non-prefix key.
2. Reconstructed tail is computed before recovery decides whether metadata is safe.
3. A committed key with higher sequence must not become invisible because metadata is stale.
4. Recovery must fail before replaying/truncating if metadata is below durable key tail.

## Acceptance Behavior Mapping

- Missing tail metadata is reconstructed from final `run_event` key.
- Matching tail metadata and final key recover without warning.
- Invalid input tail below final key returns typed `TailMismatch`.
- Missing `run_event` prefix returns typed `MissingJournal` in recovery-required mode.
- Empty `run_event` keyspace returns zero tail for pure tail scan/query mode.
- Single event key at sequence zero reconstructs tail one.
- Tail scan never crosses keyspace prefix.
- Reconstructed tail equals maximum encoded sequence plus one.
