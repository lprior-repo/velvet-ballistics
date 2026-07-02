# Test Plan — vb-xkli

STATUS: APPROVED

Primary executable gate is proof-oriented: rerun `scripts/rust-verification-gauntlet.sh proof` and reject any Kani harness failure, unsupported target in scripted commands, or weakening flag.
