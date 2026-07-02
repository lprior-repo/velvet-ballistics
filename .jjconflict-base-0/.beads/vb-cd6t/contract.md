bead_id: vb-cd6t
bead_title: quality: resolve release supply-chain blockers
phase: 3
updated_at: 2026-05-18T21:12:48.672073+00:00
attempt: 1-of-7

Requirements:
SC-1 moon run :supply-chain must pass without suppressing command failures.
SC-2 fuzz manifest must carry valid first-party license metadata.
SC-3 non-default licenses/advisories must be represented as narrow policy entries with rationale.
SC-4 cargo-vet missing coverage must remain explicit in supply-chain/config.toml.
SC-5 Known non-supply-chain verify-standard failure is not laundered; classify as DEFERRED_GLOBAL with parent blocker vb-ybi5.
STATUS: APPROVED
