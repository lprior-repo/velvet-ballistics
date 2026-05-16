# Landing Report — vb-qi37.6

STATUS: LANDED

## Main / remote evidence

- Commit: `35d4c764 fix: restore capability proof harnesses`.
- Push command: `git push origin HEAD:main`.
- Remote proof: `git ls-remote origin refs/heads/main` returned `35d4c764d96afe7df429b270fdde910dfff43690 refs/heads/main`.

## Gate evidence

- `moon ci --force` passed: `Tasks: 20 completed`; `8414 tests run: 8414 passed, 6 skipped`.
- Formal obligations passed: moon verify-proof, CapabilityLifecycle TLC configs, Verus capability model, Kani capability harnesses, capability fuzz targets.

## Bead close/sync evidence

- `bd close vb-qi37.6 --reason "Closed after State 14 landing: capability proof harness repair integrated to main at 35d4c764; moon ci --force and formal obligations passed."` succeeded.
- `bd dolt push` succeeded with `Push complete`.
- `bd show vb-qi37.6 --json` shows `status: closed`, `closed_at: 2026-05-16T03:17:57Z`.

## Notes

- Source checkout `/home/lewis/src/velvet-ballistics` has pre-existing unresolved merge state on `feature/arch-runtime-ipc`; it was not used for bead code/artifact edits. Main was landed from isolated workspace `/home/lewis/src/vb-ws/vb-qi37.6-integration`.
