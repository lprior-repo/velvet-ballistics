# Shared machine

Read this when about to run something heavy (a full build, a test suite, containers, parallel subagents, a browser-driven run) or to stop, clean up after or spawn processes, servers or browsers on a machine other agent sessions may share.

- Keep total CPU and RAM, yours plus everything already running, under ~85% of capacity. Check load before the heavy step; at the ceiling, wait until load holds below it for a couple of minutes, or shrink the job. A saturated machine manufactures timeouts and flakes that then cost a diagnosis.
- Size parallel work with an explicit worker count (`-j N`, a pool size) to the headroom actually free, since a default of "all cores" ignores what the other sessions are already using.
- Own what you started, and only that. Several agent sessions share one machine, each with its own language servers and tool processes, so a process matched by name is as likely a colleague session's: before stopping one, walk its parent chain to your own session's process id with the platform's process tool, and stop only that subtree.
- After a run that drives browsers or spawns servers, look for a survivor holding the harness's flags or profile directory and stop it once it proves to be yours. A crashed run leaves them holding ports and memory.
- Stop a backgrounded command or watch when its job is done, and prune per-run harness profile and cache directories after hours of suite runs with nothing active: they grow without bound.
