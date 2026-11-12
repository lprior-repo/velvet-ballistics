STATUS: APPROVED

Line-count scan found pre-existing oversized CLI source files: args.rs 1697, main.rs/vb.rs 4123, main_tests.rs 853, plus others. This bead changed submit ledger append behavior without introducing a new module-level dependency cycle. Classification: DEFERRED_GLOBAL follow-up to split CLI main/parser modules under separate architecture bead.
