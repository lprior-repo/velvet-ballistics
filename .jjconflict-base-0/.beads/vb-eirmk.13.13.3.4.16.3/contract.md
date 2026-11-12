# Merge runtime storage and IPC behavior

Bead: vb-eirmk.13.13.3.4.16.3. Parent: vb-eirmk.13.13.3.4.16. The frozen parent integration contract applies without weakening.

Exclusive source prefixes: crates/vb_runtime/, crates/vb_storage/, crates/vb_ipc/.

Resolve assigned semantic conflicts using both pinned sources and actual callers. Preserve all source contracts, error ordering, tests/properties/seeds and typed boundaries. Additional private helpers or existing module wiring are allowed only within the assigned domain when necessary; no public API, persisted-format or dependency expansion.



Proof/harness changes must preserve every obligation and trust marker, remain bound to real final production data, and never substitute hardcoded structural values. No source checkout, source reset, rebase, Git commit or push. Parent owns private snapshot publication and final main integration. Unassigned conflicts remain untouched. No host compilation, tests, verifier, profiler or benchmark execution before parent grants it; report static/partial limitations honestly. No /tmp.
