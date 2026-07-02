bead_id: vb-cd6t
bead_title: quality: resolve release supply-chain blockers
phase: 3
updated_at: 2026-05-18T21:12:48.672073+00:00
attempt: 1-of-7

Layer SC-L1: cargo audit evidence target/supply-chain/audit.log.
Layer SC-L2: cargo deny evidence target/supply-chain/deny.log.
Layer SC-L3: cargo vet evidence target/supply-chain/vet.log.
Layer SC-L4: moon run :supply-chain command evidence.
Layer SC-L5: moon run :verify-standard classification evidence.
