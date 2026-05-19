# vb-njju State 5 proof-writer report — PO-004 mutation retry repair-5

## Scope

Reran the approved State 5 PO-004 mutation-evidence retry sublane from isolated workspace `/home/lewis/src/femdation-vb-njju`. No production source, contracts, tests, or planned obligations were edited.

## Changed verification artifacts

- PO-004: refreshed local temp/quota preflight evidence at `target/test-output/PO-004-temp-clean-preflight.log`.
- PO-004: refreshed raw cargo-mutants stdout/stderr at `target/test-output/PO-004-cargo-mutants-admission.log`.
- PO-004: recreated blocker marker under `target/test-output/po-004-mutants/BLOCK_LOCAL_RELEASE.txt`.
- PO-004: updated this report and `.beads/vb-njju/proof-evidence.md` only.

## Commands and current status

- PO-004 `BLOCKED_INFRASTRUCTURE`: `RUSTC_WRAPPER= TMPDIR=/tmp/vb-mut cargo mutants --package vb_runtime --test-workspace true --file crates/vb_runtime/src/admission.rs --timeout 60 --jobs 2 --output target/test-output/po-004-mutants -- --test vb_ssei_verification_admission_acceptance` -> `EXIT_STATUS: 4`. Raw log shows `Found 56 mutants to test`, then `FAILED Unmutated baseline in build`; the unmutated baseline failed due to `Disk quota exceeded (os error 122)` while cargo compiled dependencies (getrandom, parking_lot_core, wait-timeout, rand_core, generator, indexmap, regex-syntax, rustix, serde_core, syn, zerocopy, blake3) and wrote build artifacts to `/tmp/vb-mut/cargo-mutants-femdation-vb-njju-*.tmp/`. No admission-branch/evidence-classification kills can be claimed. Raw: `target/test-output/PO-004-cargo-mutants-admission.log`; blocker marker: `target/test-output/po-004-mutants/BLOCK_LOCAL_RELEASE.txt`; preflight: `target/test-output/PO-004-temp-clean-preflight.log`.

## Temp/storage preflight

- Raw preflight: `target/test-output/PO-004-temp-clean-preflight.log`.
- `/tmp` is a tmpfs with 62G size limit (not a real filesystem with 900G free like `/home`).
- `df -h` shows `/tmp` at 76% use with 16G available, but the tmpfs quota is being hit by cargo-mutants build operations.
- Multiple TMPDIR paths were attempted:
  1. `TMPDIR=/tmp/vb-njju-mutants` → Disk quota exceeded
  2. `TMPDIR=/home/...workspace.../.cargo-mutants-tmp` → File name too long (cargo-mutants nested path bug)
  3. `TMPDIR=/tmp/vb-mut` → Disk quota exceeded (same root cause as #1)
- The issue is the tmpfs quota itself, not the path. All paths under `/tmp` hit the same quota.

## Preserved prior statuses only where raw logs still support them

- PO-010 previous `PASS` remains supported by `target/test-output/PO-010-vb_storage-deterministic-replay.log`: exact command selected and ran `proptests::ppi_001_deterministic_replay_invariant`; `test result: ok. 1 passed; 0 failed; 988 filtered out`.
- PO-005 `PASS_WITH_SCOPE` remains from prior run: `moon run :mutants-smoke` exits 0 with diagnostic smoke only.
- PO-017 `PASS_WITH_SCOPE` remains from prior run: `moon ci` exits 0 after 23 tasks.

## Assumptions, bounds, and blockers

- PO-004 cargo-mutants bound: 56 planned mutants in `crates/vb_runtime/src/admission.rs`; zero were executed because the unmutated baseline failed at the infrastructure level (tmpfs quota).
- The repair guide's suggested options (different TMPDIR, disable sccache, workspace-local temp) were all attempted and failed. The workspace-local temp path `/home/.../.cargo-mutants-tmp` triggered a cargo-mutants nested-path bug causing "File name too long". The /tmp paths all hit the tmpfs quota.
- BLOCKED_INFRASTRUCTURE: the system `/tmp` is a tmpfs with a 62G size limit. This is not resolvable by path changes within this sublane.
- No verifier PASS is claimed for PO-004.
- State 6 can rerun review/classification, but PO-004 remains release-blocking until cargo-mutants can execute without tmpfs quota failures.

## State 6 rerun readiness

State 6 can rerun review/classification of the new blocker evidence. Release cannot advance as fully closed because PO-004 remains `BLOCKED_INFRASTRUCTURE` with zero mutants tested.
