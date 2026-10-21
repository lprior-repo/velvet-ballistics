# Independent review scope clarification

The local5090 review executed120 lifecycle tests and1909 full library tests. The reviewed helper is135 lines, not107; the dedicated test file has17tests plus2 capability cases elsewhere (19total), not17total.

The report's valid-index mismatch observation concerns a defensive binding-helper branch. The only production caller runs dense ID validation before binding, so a mismatched ID at a valid index is rejected by the predecessor gate. Existing exact fabricated/sparse ID tests cover rejection at public admission boundaries. This observation does not establish an accepted-input behavior gap, and does not waive any defect.

Integration fixture drift is tracked in vb-bwdf0, whose separate21-field candidate compiles the whole package. The combined runtime candidate has actually executed every integration suite; two recovery error classifications remain tracked in vb-90e4k. Test-only submission fabrication is in shard/lifecycle/chunk_001.rs, tracked vb-8iilq; the with-contract paths tested here do not take those replacements. Neither open bead is waived or accepted by this review.

This bounded review supports the admission change's ordered validation, real successful Do output, and journal-preserving rejections. It is not full recovery acceptance, full runtime package acceptance, broad CI closure, or v1 completion. Bead vb-3l40a remains in progress while broader prior findings are reconciled.
