# Retained current-main taxonomy fixture

Production source commit: f366003714753937573e04650bf63cc4e5beb8ea. Git tree: d6c13f925804b58f98260e012c6f2c0c5f4afd21. Only workspace integration test files changed. No production source, public API, schema, dependencies, guards or executor version changed.

The final archive is retained-v3. Its complete raw Fjall directories were closed before copying. It contains genuinely issued Strict None/Safe and registered Writes/KeyRequired accepted artifacts, run-admission events, pending ActionTicket, scheduled ABI digest and full journal prefix. The registered artifact includes its actual manifest, executor version, receipt schema and conformance binding in pack.postcard. Its register contains one actual committed effect and receipt; raw-receipt-rows.postcard exports the exact key and receipt-record bytes, and the opaque receipt digest/value/ordinal observations are separately retained. Sends, Creates and Destroys are genuine Journaled accepted artifacts kept cold: the current registered catalog only implements Writes, so these do not receive execution or Strict registered authority.

The test reopens a private copy of every database. It compares retained artifact and event bytes, checks the original ABI digest, resumes the identical pending ticket, and reconciles the retained registered receipt while effect_count remains 1 and value remains 42. It never deserializes an authority token or constructs a replacement accepted authority. The optional VB_TAXONOMY_ARCHIVE must name an existing archive; a missing path fails instead of generating replacement data. With the variable absent, the same test exercises fresh issuer APIs in disposable scratch beneath the explicitly configured persistent TMPDIR.

Baseline exact production source run: baseline.log, 33/33 pass. Final producer run: producer-v3.log, 34/34 pass. Final same-archive replay: replay-final.log. V1 is a failed producer attempt, preserved separately and excluded from the accepted archive: execution added a PolicyDecisionEvent after the initial schedule snapshot, and the event equality assertion caught this. V2 corrected capture ordering and passed; V3 adds explicit raw receipt rows. retained-v1 and retained-v2 are historical scratch, not accepted fixture inputs.

Run from this workspace root with all artifacts and scratch remaining here:

```sh
(cd .evidence/taxonomy60 && sha256sum -c retained-v3.sha256)
TMPDIR="$PWD/.evidence/taxonomy60/tmp" CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 NEXTEST_TEST_THREADS=4 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings VB_TAXONOMY_ARCHIVE="$PWD/.evidence/taxonomy60/retained-v3" timeout 10m rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet --all-features -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery --test vb_7x0ol_probe_registered_executor
(cd .evidence/taxonomy60 && sha256sum -c retained-v3.sha256)
```

The native gate is the pinned nextest command from .moon/tasks/all.yml:test, restricted to the two named integration targets and their owning package to avoid a broad duplicate build. moon ci was not run. This is baseline retention evidence, not proof closure, taxonomy design acceptance or a performance claim. The original run origin is real clock data; replay remains subject to the current bounded run-lifetime policy.

Skills/references read: jj, beads, functional-rust, holzman-rust, test-writer; rules/workflow.md, rules/testing.md, rules/commit-messages.md; holzman-rust/references/nasa-jpl-standards.md and runtime-performance-architecture.md; test-writer/references/rust-test-ecosystem.md. Test-only allocations and bounded file-tree traversal occur outside production runtime paths.
