# vb-kyyf TLA Report

STATUS: APPROVED

## Obligation
- ID: `PO-008`
- Requirement: `TLA-KYYF-001`
- Module: `VbKyyfReplayDeterminism`
- Artifact: `verification/tla/VbKyyfReplayDeterminism.tla`
- Config: `verification/tla/VbKyyfReplayDeterminism.cfg`
- Layer: `tla-plus`
- Scope: `protocol`

## Command
```bash
JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp' tlc -workers 32 -metadir /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla
```

## Result
- Exit: `0`
- Classification: `PASS`

## Raw Evidence
```text
Picked up JAVA_TOOL_OPTIONS: -Djava.io.tmpdir=/home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Warning: Please run the Java VM which executes TLC with a throughput optimized garbage collector by passing the "-XX:+UseParallelGC" property.
(Use the -nowarning option to disable this warning.)
Running breadth-first search Model-Checking with fp 125 and seed 8972946055542272879 with 32 workers on 32 cores with 30688MB heap and 64MB offheap memory [pid: 82750] (Linux 7.0.3-arch1-2 amd64, Oracle Corporation 26.0.1 x86_64, MSBDiskFPSet, DiskStateQueue).
Parsing file /home/lewis/src/bd-vb-kyyf-bdd/verification/tla/VbKyyfReplayDeterminism.tla
Parsing file /home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp/Naturals.tla
Parsing file /home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp/Sequences.tla
Parsing file /home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp/FiniteSets.tla
Semantic processing of module Naturals
Semantic processing of module Sequences
Semantic processing of module FiniteSets
Semantic processing of module VbKyyfReplayDeterminism
Starting... (2026-05-18 15:38:43)
Implied-temporal checking--satisfiability problem has 3 branches.
Computing initial states...
Computed 2 initial states...
Computed 4 initial states...
Finished computing initial states: 6 distinct states generated at 2026-05-18 15:38:43.
Checking 3 branches of temporal properties for the current state space with 270659 total distinct states at (2026-05-18 15:38:46)
Finished checking temporal properties in 00s at 2026-05-18 15:38:46
Progress(6) at 2026-05-18 15:38:46: 151,484 states generated (151,484 s/min), 145,186 distinct states found (145,186 ds/min), 138,943 states left on queue.
Checking 3 branches of temporal properties for the current state space with 6555811 total distinct states at (2026-05-18 15:39:46)
Finished checking temporal properties in 36s at 2026-05-18 15:40:23
Progress(8) at 2026-05-18 15:40:23: 5,781,065 states generated (5,629,581 s/min), 4,544,939 distinct states found (4,399,753 ds/min), 3,786,253 states left on queue.
Progress(8) at 2026-05-18 15:41:23: 14,515,152 states generated (8,734,087 s/min), 9,109,897 distinct states found (4,564,958 ds/min), 6,380,915 states left on queue.
Progress(9) at 2026-05-18 15:42:23: 24,386,716 states generated (9,871,564 s/min), 12,779,627 distinct states found (3,669,730 ds/min), 6,528,755 states left on queue.
Checking 3 branches of temporal properties for the current state space with 36914422 total distinct states at (2026-05-18 15:43:23)
Finished checking temporal properties in 02min 05s at 2026-05-18 15:45:29
Progress(9) at 2026-05-18 15:45:29: 37,747,634 states generated (13,360,918 s/min), 16,029,566 distinct states found (3,249,939 ds/min), 3,751,057 states left on queue.
Progress(9) at 2026-05-18 15:45:38: 42,907,696 states generated, 16,483,704 distinct states found, 0 states left on queue.
Checking 3 branches of temporal properties for the complete state space with 49451112 total distinct states at (2026-05-18 15:45:38)
Finished checking temporal properties in 02min 00s at 2026-05-18 15:47:38
Model checking completed. No error has been found.
  Estimates of the probability that TLC did not check all reachable states
  because two distinct states had the same fingerprint:
  calculated (optimistic):  val = 2.4E-5
  based on the actual fingerprints:  val = 1.1E-5
42907696 states generated, 16483704 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 9.
The average outdegree of the complete state graph is 1 (minimum is 0, the maximum 31 and the 95th percentile is 4).
Finished in 08min 57s at (2026-05-18 15:47:40)
```
