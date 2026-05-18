bead_id: vb-cd6t
bead_title: quality: resolve release supply-chain blockers
phase: 1
updated_at: 2026-05-18T21:08:42.031279+00:00
attempt: 1-of-7

Baseline evidence sources before repair:
- bd show vb-cd6t: blocker is moon run :supply-chain from parent vb-qi37.23 State 11.
- Parent raw log: /home/lewis/src/go-skill-vb-qi37-23-current/target/vb-qi37.23-evidence/resume-20260518T205451Z/supply-chain.log
- Observed failures in parent log:
  - cargo-deny license rejected NCSA for libfuzzer-sys 0.4.12.
  - cargo-deny license rejected MPL-2.0 for resvg 0.42.0 and usvg 0.42.0.
  - velvet-ballastics-fuzz synthesized manifest unlicensed.
  - RUSTSEC-2025-0057 fxhash 0.2.1 unmaintained advisory.
  - duplicate crate warnings are nonfatal warnings in same log.
Classification from parent: BLOCK_RELEASE / REQUIRED_OBLIGATION_FAIL.
