# Exact raw-log provenance binding

A closure row must bind its complete command, cwd, timestamp and exit_code values to labeled raw-log fields. A longer value sharing a prefix or a label embedded inside another line must fail with the existing provenance diagnostic and exit 1. Exact bindings retain LF, CRLF, CR and end-of-file boundaries and literal multiline values. No schema, persisted field layout, dependency, public CLI or gate changes.

First reproduce the public false PASS and make public CLI regression cases fail on the old implementation. Repair the shared string/exit-code binding predicate, then run the unchanged suites plus new positive/negative cases, strict source checks and independent CODE/TEST reviews. This hardens an inherited Python weakness; it does not justify historical evidence or waive acceptance gates.
