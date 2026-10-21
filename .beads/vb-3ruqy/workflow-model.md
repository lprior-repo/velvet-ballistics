# Workflow model

Compile: parse declarations -> reject unsupported schema keys -> reserve complete numeric layout -> bind names -> lower every consumer -> validate typed numeric IR -> bind artifact identity. All failure edges terminate with a typed diagnostic before publishing an artifact.

Submit: select explicit legacy or owned transport version -> decode bounded payload -> distinguish omitted/supplied -> validate kinds/ownership/budgets -> freeze effective values -> prepare versioned input/artifact binding -> atomically publish required durable records. Failed preparation exposes no accepted run. No default application is part of this workflow.

Run: instantiate destination ValueStore -> materialize validated owned values -> seed frame once -> deterministic drive until suspension/terminal. Recovery of a never-driven accepted run replays the same materialization from retained inputs; recovery of an executed/snapshotted frame restores its execution state rather than overwriting it with initial inputs. Corrupt or unavailable backing produces a typed failure before drive.

No new async tasks, retries or channels are needed for the pure resolution phase. Existing admission serialization and write-lock/WAL boundaries remain storage-owner responsibilities.
