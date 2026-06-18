bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
updated_at: 2026-06-17T20:00:00.000000+00:00
attempt: 1-of-7

STATUS: GLOBAL_READINESS_CAPTURED
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/femdation-tier-a-0-002

Scope classification:
- touched_production_rust: false
- new_production_rust: false
- new_proof_obligations: false
- required_proof_lanes: []
- required_machine_gates: []

Scope note: bead scope is ci-gate-installation. No production Rust code change is required; no Verus, Kani, Flux, Loom, Miri, proptest, fuzz, or cargo-mutants lane is required. The CI gate installation lives at the scripts/ + .moon/tasks/ boundary. Existing infrastructure is preserved.

BLOCK_GLOBAL pre-edit checklist: the 5 P0s in tier-a-0 wave are mutually independent. None blocks another. The Tier A master amendment (tier-a-0-005) is a pure doc append. No prior BLOCK_GLOBAL repo-wide failure is in scope for this bead.

Local precondition: jj 0.41.0 at /usr/bin/jj. Moon v2 in .moon/. The bead scope is the Tier A wave-0 master amendment and 4 CI gates; the global readiness is acceptable for State 2 (explore) entry.
