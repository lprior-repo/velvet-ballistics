# Machine Gate Report — vb-qi37.5.3

## Gate: cargo test -p vb_storage

```
$ cargo test -p vb_storage 2>&1
   Compiling vb_storage v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.28s
     Running tests/proptests.rs

running 1015 tests
test result: ok. 1015 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.80s

     Running unittests src/lib.rs

running 29 tests
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running unittests src/keys.rs

running 4 tests
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/recovery_integration.rs

running 16 tests
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/replay_resume.rs

running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/vb_h6ix_integration.rs

running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Doc-tests vb_storage

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

RESULT: PASS — 1074 tests pass, 0 failed
```

## Gate: cargo clippy -p vb_storage

```
$ cargo clippy -p vb_storage --all-features -- -D warnings 2>&1
   Compiling vb_storage v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.28s

RESULT: PASS — 0 warnings
```

## Gate: cargo fmt --check

```
$ cargo fmt --check 2>&1
(no output — no diffs)

RESULT: PASS — formatting compliant
```

## Gate: cargo build -p vb_storage

```
$ cargo build -p vb_storage 2>&1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s

RESULT: PASS — builds cleanly
```

## Summary

| Gate | Result |
|------|--------|
| cargo test -p vb_storage | PASS (1074 tests) |
| cargo clippy -p vb_storage | PASS (0 warnings) |
| cargo fmt --check | PASS |
| cargo build -p vb_storage | PASS |
