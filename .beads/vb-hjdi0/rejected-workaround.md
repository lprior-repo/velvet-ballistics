# Real producer replay blocker

Restored public-API probes revealed ActionAbiMismatch on actual completed runtime events. The schedule event has a real nonzero ABI, but completion carries zero; recover_full_journal also requires a legacy ActionScheduled variant when expected ABI is nonempty. The prior local worker replaced public replay with manual tracker folds, which is rejected as an oracle weakening. Preserve real runtime fixtures, restore public replay_journal and fix production under vb-hjdi0. Never accept zero ABI or bypass verification.

Raw original probe logs were copied from /tmp byte for byte. Three vb_compile compile-probe logs were moved from repository root into this bead; JSON original execution locations remain unchanged. These interrupted author probes are diagnostics, not passing closure evidence. Fresh immutable runner results are required.
