# Forced canonical CI execution

Command: systemd-run --user --scope --collect -p MemoryHigh=20G -p MemoryMax=24G -p MemorySwapMax=0 env RUSTC_WRAPPER= CARGO_BUILD_JOBS=2 timeout --kill-after=10 180 moon ci --force --concurrency 2
Source: eaa1062c, isolated verified Git/JJ root. Exit124 after180seconds. Moon summary:13completed,14failed,8skipped, interrupted. Full gate did not pass or finish. Raw moon-ci.log and ciReport.json retained.

Current failures include8fuzz constructor input_slots omissions (vb-bwdf0),5source-length failures (vb-idxsn),4runtime lint errors (vb-3dlcn), pinned Beads metadata port, stale sccache references in Kani/Loom tasks, existing production mirror drift and Flux/Rust syntax formatting issue. Supply-chain findings remain advisory under existing master waiver; remote cache upload failures were warnings. No newwaiver, proof closure, exhaustive fuzz or benchmark speed claim follows from this run. Root goal remains active.
