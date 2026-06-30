bead_id: vb-cd6t
bead_title: quality: resolve release supply-chain blockers
phase: 11
updated_at: 2026-05-18T21:12:48.672073+00:00
attempt: 1-of-7

STATUS: PASS
Commands:
- moon run :supply-chain => PASS, Tasks: 1 completed, 3s 524ms.
- rustup run nightly-2026-04-28 cargo vet --store-path supply-chain --locked --verbose error => PASS, Vetting Succeeded (361 exempted).
- moon run :verify-standard => FAIL on GATE-IGNORED-FALLIBLE-RESULTS in crates/vb_storage/src/kani_recovery_hydrate.rs lines 78 and 111; classified DEFERRED_GLOBAL because parent evidence already tracks blocker vb-ybi5 and delivery scope is supply-chain policy/config.
Raw logs: target/supply-chain/audit.log, deny.log, vet.log, machete.log.
