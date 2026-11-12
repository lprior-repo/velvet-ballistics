# Attach the existing command documentation

Base is immutable bd4a5233e6994237effb7312c0a6e80ed2e326ed. The actual strict standalone caller command exited101 because xtask/src/proof/commands.rs has one blank line between its existing documentation block and commands_for_obligation.

Remove only that blank line. Preserve every nonblank source line, documentation word, function, test, command, oracle, registration and ordering. Do not edit gates, suppressions, settings, dependencies, registry, proof math, other source files or the parent candidate.

Rerun the exact saved strict-callers-r2 native argv with the same pinned nightly, all features, all source lint flags, standalone manifest mirror, owned target, jobs2, wrappers disabled and24GiB cap. Require actual101to0 and pinned file-only rustfmt check0. Freeze a two-file source-plus-contract successor for independent delta CODE and TEST; no broad validation, commit, push, closure or other-workspace edits.
