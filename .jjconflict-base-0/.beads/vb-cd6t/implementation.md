bead_id: vb-cd6t
bead_title: quality: resolve release supply-chain blockers
phase: 10
updated_at: 2026-05-18T21:12:48.672073+00:00
attempt: 1-of-7

Changed files:
- deny.toml: scoped license exceptions for libfuzzer-sys NCSA, resvg/usvg MPL-2.0; documented RUSTSEC-2025-0057 ignore with Makepad/fxhash no-safe-upgrade rationale.
- fuzz/Cargo.toml: added MIT OR Apache-2.0 license.
- supply-chain/config.toml: cargo-vet formatted exemptions for currently locked missing coverage surfaced after deny repair.
No production Rust code changed.
