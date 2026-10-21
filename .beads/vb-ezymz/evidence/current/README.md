# vb-ezymz admission candidate evidence

Scope: typed CLI retention, canonical input identity, atomic storage submission, explicit legacy receipt decoding. This is a candidate for independent code and test review; no approval or bead closure is claimed. Artifact preparation/verifier dependency is commit 948f19cb7cd92afec3d95e9fb285216cc6689d41.

Each named run has exact `.command`, `.workdir`, `.stdout`, `.stderr`, `.exit` files. Earlier failed runs remain as history; they are superseded only by the named successful reruns below.

| Run | Actual result |
| --- | --- |
| storage-schema-check | Storage/runtime/CLI production check exit 0 |
| storage-input-final-4 | 13 passed, 0 failed, 0 ignored, 1599 filtered; includes 64 seeded i64 property cases |
| storage-codec-final | 174 passed, 0 failed, 0 ignored, 1438 filtered |
| cli-admission-final | CLI boundary 6 passed and retained execution 2 passed; no ignored/filtered cases |
| manifest-final | Manifest 23 passed and doctor 48 passed; no ignored/filtered cases |
| storage-keyspaces-final-2 | 5 passed, 0 failed, 0 ignored, 1607 filtered |
| source-clippy-final-2 | Production storage+CLI strict source lint and dependency compilation exit 0 |
| source-diff-check | Rust/TOML/Markdown diff whitespace check exit 0 |

CLI execution coverage submits MIN,-7,0,41,MAX under none/journaled/strict, deletes original source/input files and reopens twice. The persisted independent artifact verifier reconstructs numeric IR; production seed_input_slots and run_until_blocked return the exact retained value. Durable modes validate the committed input-digest receipt. None has no acceptance events and makes no durable-run promise. This does not cover vb-netau's full runtime recovery loader; that owner supplies integration and independent review.

Receipt tests cover canonical input-order independence, slot/value/type sensitivity, duplicate rejection, rehashed envelope tampering, internally consistent replacement inputs against the original receipt, legacy-unbound/run/artifact/digest mismatch, new stable tag36, historical stable tag24 and schema-one ordinal1, trailing bytes and envelope/payload kind mismatch. Recovery admission summary preserves the bound digest.

Atomic failure coverage uses the repository's injected precommit fault in each policy, inspects all records/events/status+workflow indexes, retries successfully and rejects run collision. Actual Fjall WAL batch atomicity and SyncAll semantics are external storage guarantees, not exhaustive injected hardware faults. Journaled retains its acknowledged-loss window.

Historical failed fixture attempts and test compile mistakes remain in this directory with exit101. `storage-keyspaces-final` caught an order regression; the implementation now appends run_inputs after the nine existing manifest entries. The final focused assertions pass. Raw early tee logs contain an ending blank line; they are preserved unchanged. No full moon ci or independent review was executed by this specialist; the integration lane owns those gates.
