bead_id: vb-cd6t
bead_title: quality: resolve release supply-chain blockers
phase: 2
updated_at: 2026-05-18T21:12:48.672073+00:00
attempt: 1-of-7

Scope map:
- deny.toml controls cargo-deny licenses/advisory policy.
- fuzz/Cargo.toml is the synthesized fuzz crate manifest source.
- supply-chain/config.toml is the cargo-vet store.
- .moon/tasks/all.yml supply-chain runs cargo audit, cargo deny, cargo vet, geiger reports, cargo machete.
Parent evidence log identified license, unlicensed manifest, and fxhash advisory failures.
