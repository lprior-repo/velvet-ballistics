bead_id: vb-zrop
bead_title: quality: fix verify-standard ignored fallible result gate
phase: 3
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# Contract

REQ-001: `moon run :verify-standard` must not fail `GATE-IGNORED-FALLIBLE-RESULTS` for the scoped source paths.
REQ-002: No gate weakening, allowlist broadening, or scanner exemption may be used.
REQ-003: Fallible cleanup/setup/write results must be handled explicitly or converted to a narrow documented best-effort exception that the existing scanner accepts.
REQ-004: No runtime behavior, dependency, public API, feature flag, or build-script change is intended.

Preconditions: baseline reproduces ignored fallible results in `.beads/vb-zrop/baseline-verify-standard.log`.
Postconditions: focused scanner and `moon run :verify-standard` exit 0.
Invariants: safe Rust policy; no new unsafe/panic/todo/unimplemented/dbg/ignored Result introduced in touched code.
Error taxonomy: verifier failure remains BLOCK_RELEASE / REQUIRED_OBLIGATION_FAIL.
