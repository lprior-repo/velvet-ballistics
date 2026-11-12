# Waiver Candidates: vb-zioy

## Status

**No waiver candidates.**

## Rationale

This bead has no non-behavior exceptions that would warrant a waiver. Every contract clause is behavior-affecting (diagnostic fidelity directly impacts user experience), and all obligations are planned for verification through tests or compile-time checks.

The delivery scope explicitly states `formal_verification_needed: false`, but this does not waive any behavioral obligation. It only means the specific risks (temporal, arithmetic, unsafe, concurrency) that would require formal verifiers are absent.

## Explicit Non-Waiver Decisions

| Verifier | Seed | Decision |
|----------|------|----------|
| TLA+ | All | `not_applicable` with concrete evidence — no temporal properties |
| Verus | All | `not_applicable` with concrete evidence — no arithmetic/typestate |
| Kani | All | `not_applicable` with concrete evidence — no panic/overflow risk |
| Flux | All | `not_applicable` with concrete evidence — no refinement types |
| Loom | All | `not_applicable` with concrete evidence — no concurrency |
| Miri | All | `not_applicable` with concrete evidence — no unsafe code |
| cargo-fuzz | All | `not_applicable` with concrete evidence — no parsing change |

All of the above are classified as `risk_absent` per the lane decision guide, not waived.
