Kani Rust Verifier 0.67.0 (cargo plugin)
warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:1966:25
     |
1966 |                     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
     |                         ----^^^^^
     |                         |
     |                         help: remove this `mut`
     |
     = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:1981:25
     |
1981 |                     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
     |                         ----^^^^^
     |                         |
     |                         help: remove this `mut`

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:1999:25
     |
1999 |                     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
     |                         ----^^^^^
     |                         |
     |                         help: remove this `mut`

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:2017:29
     |
2017 |                         let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
     |                             ----^^^^^
     |                             |
     |                             help: remove this `mut`

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:2026:29
     |
2026 |                         let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
     |                             ----^^^^^
     |                             |
     |                             help: remove this `mut`

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:2038:29
     |
2038 |                         let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, step_count, 1);
     |                             ----^^^^^
     |                             |
     |                             help: remove this `mut`

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:2055:25
     |
2055 |                     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
     |                         ----^^^^^
     |                         |
     |                         help: remove this `mut`

warning: variable does not need to be mutable
    --> crates/vb_core/src/frame.rs:2076:25
     |
2076 |                     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
     |                         ----^^^^^
     |                         |
     |                         help: remove this `mut`

warning: variable does not need to be mutable
  --> crates/vb_core/src/kani_taint.rs:76:9
   |
76 |     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
   |         ----^^^^^
   |         |
   |         help: remove this `mut`

warning: variable does not need to be mutable
  --> crates/vb_core/src/kani_taint.rs:99:9
   |
99 |     let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
   |         ----^^^^^
   |         |
   |         help: remove this `mut`

warning: field `resource` is never read
    --> crates/vb_core/src/budget.rs:1575:21
     |
1575 |         Underflow { resource: &'static str },
     |         ---------   ^^^^^^^^
     |         |
     |         field in this variant
     |
     = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: Found the following unsupported constructs:
             - caller_location (1)
             - foreign function (1)
         
         Verification will fail if one or more of these constructs is reachable.
         See https://model-checking.github.io/kani/rust-feature-support.html for more details.

    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
Checking harness frame::frame_kani_harnesses::kani_step_state...
CBMC 6.8.0 (cbmc-6.8.0)
CBMC version 6.8.0 (cbmc-6.8.0) 64-bit x86_64 linux
Reading GOTO program from file /home/lewis/src/tmp_build/vb-qi37.4.2-cargo-target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_core-c38eee64bd9b50d5__RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses15kani_step_state.out
Generating GOTO Program
Adding CPROVER library (x86_64)
Removal of function pointers and virtual functions
Generic Property Instrumentation
Running with 16 object bits, 48 offset bits (user-specified)
Starting Bounded Model Checking
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 1 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 2 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 3 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 4 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 5 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 6 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 7 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 8 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.1 iteration 1 file crates/vb_core/src/frame.rs line 1946 column 9 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 1 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 2 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 3 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 4 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 5 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 6 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 7 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 8 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.1 iteration 2 file crates/vb_core/src/frame.rs line 1946 column 9 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 1 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 2 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 3 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 4 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 5 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 6 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 7 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 8 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.1 iteration 3 file crates/vb_core/src/frame.rs line 1946 column 9 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 1 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 2 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 3 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 4 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 5 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 6 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 7 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.0 iteration 8 file crates/vb_core/src/frame.rs line 1947 column 13 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Unwinding loop _RNvNtNtCs1wqsg5Qkq1s_7vb_core5frame20frame_kani_harnesses39validate_transition_terminal_blocks_all.1 iteration 4 file crates/vb_core/src/frame.rs line 1946 column 9 function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all thread 0
Runtime Symex: 0.573146s
size of program expression: 27804 steps
slicing removed 19461 assignments
Generated 3582 VCC(s), 251 remaining after simplification
Runtime Postprocess Equation: 0.00941784s
Passing problem to propositional reduction
converting SSA
Runtime Convert SSA: 0.0516156s
Running propositional reduction
Post-processing
Runtime Post-process: 5.31e-06s
Solving with CaDiCaL 2.0.0
85905 variables, 92661 clauses
SAT checker: instance is SATISFIABLE
Runtime Solver: 0.0039668s
Runtime decision procedure: 0.0566916s
Running propositional reduction
Solving with CaDiCaL 2.0.0
85906 variables, 92662 clauses
SAT checker: instance is UNSATISFIABLE
Runtime Solver: 0.00115215s
Runtime decision procedure: 0.00126111s

RESULTS:
Check 1: core::panicking::panic_nounwind_fmt::runtime.unsupported_construct.1
	 - Status: SUCCESS
	 - Description: "call to foreign "Rust" function `_RNvCs1hStedNDpZ2_7___rustc17rust_begin_unwind` is not currently supported by Kani. Please post your example at https://github.com/model-checking/kani/issues/new/choose"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/panicking.rs:110:17 in function core::panicking::panic_nounwind_fmt::runtime

Check 2: kani::mem::cbmc::same_allocation.unsupported_construct.1
	 - Status: SUCCESS
	 - Description: "Kani does not support reasoning about pointer to unallocated memory"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function kani::mem::cbmc::same_allocation

Check 3: <usize as kani::rustc_intrinsics::ToISize>::to_isize.unreachable.1
	 - Status: SUCCESS
	 - Description: "unreachable code"
	 - Location: ../../../../runner/work/kani/kani/library/kani_core/src/models.rs:176:17 in function <usize as kani::rustc_intrinsics::ToISize>::to_isize

Check 4: <usize as kani::rustc_intrinsics::ToISize>::to_isize.safety_check.1
	 - Status: SUCCESS
	 - Description: "Offset value overflows isize"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function <usize as kani::rustc_intrinsics::ToISize>::to_isize

Check 5: <usize as kani::rustc_intrinsics::ToISize>::to_isize.assertion.1
	 - Status: SUCCESS
	 - Description: "internal error: entered unreachable code"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function <usize as kani::rustc_intrinsics::ToISize>::to_isize

Check 6: frame::frame_kani_harnesses::step_state_from_u8.assertion.1
	 - Status: SUCCESS
	 - Description: "attempt to calculate the remainder with a divisor of zero"
	 - Location: crates/vb_core/src/frame.rs:1336:15 in function frame::frame_kani_harnesses::step_state_from_u8

Check 7: frame::frame_kani_harnesses::step_state_from_u8.arithmetic_overflow.1
	 - Status: SUCCESS
	 - Description: "attempt to calculate the remainder with a divisor of zero"
	 - Location: crates/vb_core/src/frame.rs:1336:15 in function frame::frame_kani_harnesses::step_state_from_u8

Check 8: core::num::<impl usize>::unchecked_sub.arithmetic_overflow.1
	 - Status: SUCCESS
	 - Description: "attempt to compute `unchecked_sub` which would overflow"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/num/uint_macros.rs:908:17 in function core::num::<impl usize>::unchecked_sub

Check 9: kani::rustc_intrinsics::offset::<frame::StepState, *const frame::StepState, usize>.safety_check.1
	 - Status: SUCCESS
	 - Description: "Offset in bytes overflows isize"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function kani::rustc_intrinsics::offset::<frame::StepState, *const frame::StepState, usize>

Check 10: kani::rustc_intrinsics::offset::<frame::StepState, *const frame::StepState, usize>.safety_check.2
	 - Status: SUCCESS
	 - Description: "Offset result and original pointer must point to the same allocation"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function kani::rustc_intrinsics::offset::<frame::StepState, *const frame::StepState, usize>

Check 11: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.unreachable.1
	 - Status: SUCCESS
	 - Description: "unreachable code"
	 - Location: crates/vb_core/src/lib.rs:0:0 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 12: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.assertion.1
	 - Status: SUCCESS
	 - Description: "terminal->self allowed"
	 - Location: crates/vb_core/src/frame.rs:1950:21 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 13: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.assertion.2
	 - Status: SUCCESS
	 - Description: "terminal->other blocked"
	 - Location: crates/vb_core/src/frame.rs:1952:21 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 14: std::panic::Location::<'_>::caller.unsupported_construct.1
	 - Status: SUCCESS
	 - Description: "caller_location is not currently supported by Kani. Please post your example at https://github.com/model-checking/kani/issues/374"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/panic/location.rs:147:9 in function std::panic::Location::<'_>::caller

Check 15: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.safety_check.1
	 - Status: SUCCESS
	 - Description: "misaligned pointer to reference cast: address must be a multiple of its type's alignment"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:18 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 16: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.safety_check.2
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:18 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 17: std::fmt::Arguments::<'_>::from_str.assertion.1
	 - Status: UNREACHABLE
	 - Description: "attempt to shift left with overflow"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/fmt/mod.rs:820:38 in function std::fmt::Arguments::<'_>::from_str

Check 18: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.1
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1359:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 19: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.2
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1361:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 20: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.3
	 - Status: SUCCESS
	 - Description: "P->R"
	 - Location: crates/vb_core/src/frame.rs:1362:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 21: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.4
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1367:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 22: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.5
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1369:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 23: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.6
	 - Status: SUCCESS
	 - Description: "P->S"
	 - Location: crates/vb_core/src/frame.rs:1370:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 24: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.7
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1375:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 25: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.8
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1377:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 26: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.9
	 - Status: SUCCESS
	 - Description: "P->F"
	 - Location: crates/vb_core/src/frame.rs:1378:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 27: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.10
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1383:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 28: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.11
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1385:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 29: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.12
	 - Status: SUCCESS
	 - Description: "P->K"
	 - Location: crates/vb_core/src/frame.rs:1386:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 30: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.13
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1391:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 31: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.14
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1393:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 32: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.15
	 - Status: SUCCESS
	 - Description: "P->C"
	 - Location: crates/vb_core/src/frame.rs:1394:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 33: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.16
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1399:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 34: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.17
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1401:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 35: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.18
	 - Status: SUCCESS
	 - Description: "P->W!"
	 - Location: crates/vb_core/src/frame.rs:1402:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 36: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.19
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1407:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 37: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.20
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1409:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 38: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.21
	 - Status: SUCCESS
	 - Description: "P->A!"
	 - Location: crates/vb_core/src/frame.rs:1410:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 39: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.22
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1415:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 40: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.23
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1417:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 41: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.24
	 - Status: SUCCESS
	 - Description: "P->P"
	 - Location: crates/vb_core/src/frame.rs:1418:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 42: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.25
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1426:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 43: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.26
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1428:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 44: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.27
	 - Status: SUCCESS
	 - Description: "R->P!"
	 - Location: crates/vb_core/src/frame.rs:1429:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 45: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.28
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1434:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 46: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.29
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1436:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 47: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.30
	 - Status: SUCCESS
	 - Description: "R->R"
	 - Location: crates/vb_core/src/frame.rs:1437:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 48: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.31
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1442:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 49: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.32
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1444:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 50: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.33
	 - Status: SUCCESS
	 - Description: "R->S"
	 - Location: crates/vb_core/src/frame.rs:1445:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 51: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.34
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1450:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 52: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.35
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1452:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 53: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.36
	 - Status: SUCCESS
	 - Description: "R->F"
	 - Location: crates/vb_core/src/frame.rs:1453:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 54: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.37
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1458:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 55: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.38
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1460:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 56: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.39
	 - Status: SUCCESS
	 - Description: "R->K"
	 - Location: crates/vb_core/src/frame.rs:1461:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 57: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.40
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1466:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 58: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.41
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1468:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 59: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.42
	 - Status: SUCCESS
	 - Description: "R->W"
	 - Location: crates/vb_core/src/frame.rs:1469:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 60: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.43
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1474:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 61: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.44
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1476:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 62: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.45
	 - Status: SUCCESS
	 - Description: "R->A"
	 - Location: crates/vb_core/src/frame.rs:1477:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 63: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.46
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1482:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 64: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.47
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1484:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 65: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.48
	 - Status: SUCCESS
	 - Description: "R->C"
	 - Location: crates/vb_core/src/frame.rs:1485:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 66: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.49
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1493:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 67: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.50
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1495:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 68: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.51
	 - Status: SUCCESS
	 - Description: "X->P!"
	 - Location: crates/vb_core/src/frame.rs:1496:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 69: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.52
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1501:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 70: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.53
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1503:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 71: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.54
	 - Status: SUCCESS
	 - Description: "X->R!"
	 - Location: crates/vb_core/src/frame.rs:1504:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 72: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.55
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1509:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 73: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.56
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1511:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 74: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.57
	 - Status: SUCCESS
	 - Description: "X->F!"
	 - Location: crates/vb_core/src/frame.rs:1512:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 75: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.58
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1517:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 76: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.59
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1519:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 77: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.60
	 - Status: SUCCESS
	 - Description: "X->K!"
	 - Location: crates/vb_core/src/frame.rs:1520:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 78: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.61
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1525:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 79: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.62
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1527:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 80: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.63
	 - Status: SUCCESS
	 - Description: "X->W!"
	 - Location: crates/vb_core/src/frame.rs:1528:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 81: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.64
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1533:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 82: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.65
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1535:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 83: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.66
	 - Status: SUCCESS
	 - Description: "X->A!"
	 - Location: crates/vb_core/src/frame.rs:1536:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 84: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.67
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1541:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 85: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.68
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1543:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 86: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.69
	 - Status: SUCCESS
	 - Description: "X->C!"
	 - Location: crates/vb_core/src/frame.rs:1544:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 87: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.70
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1549:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 88: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.71
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1551:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 89: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.72
	 - Status: SUCCESS
	 - Description: "X->X"
	 - Location: crates/vb_core/src/frame.rs:1552:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 90: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.73
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1560:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 91: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.74
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1562:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 92: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.75
	 - Status: SUCCESS
	 - Description: "X->P!"
	 - Location: crates/vb_core/src/frame.rs:1563:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 93: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.76
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1568:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 94: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.77
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1570:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 95: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.78
	 - Status: SUCCESS
	 - Description: "X->R!"
	 - Location: crates/vb_core/src/frame.rs:1571:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 96: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.79
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1576:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 97: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.80
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1578:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 98: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.81
	 - Status: SUCCESS
	 - Description: "X->S!"
	 - Location: crates/vb_core/src/frame.rs:1579:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 99: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.82
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1584:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 100: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.83
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1586:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 101: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.84
	 - Status: SUCCESS
	 - Description: "X->K!"
	 - Location: crates/vb_core/src/frame.rs:1587:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 102: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.85
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1592:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 103: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.86
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1594:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 104: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.87
	 - Status: SUCCESS
	 - Description: "X->W!"
	 - Location: crates/vb_core/src/frame.rs:1595:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 105: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.88
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1600:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 106: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.89
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1602:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 107: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.90
	 - Status: SUCCESS
	 - Description: "X->A!"
	 - Location: crates/vb_core/src/frame.rs:1603:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 108: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.91
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1608:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 109: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.92
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1610:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 110: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.93
	 - Status: SUCCESS
	 - Description: "X->C!"
	 - Location: crates/vb_core/src/frame.rs:1611:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 111: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.94
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1616:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 112: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.95
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1618:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 113: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.96
	 - Status: SUCCESS
	 - Description: "X->X"
	 - Location: crates/vb_core/src/frame.rs:1619:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 114: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.97
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1627:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 115: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.98
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1629:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 116: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.99
	 - Status: SUCCESS
	 - Description: "X->P!"
	 - Location: crates/vb_core/src/frame.rs:1630:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 117: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.100
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1635:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 118: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.101
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1637:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 119: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.102
	 - Status: SUCCESS
	 - Description: "X->R!"
	 - Location: crates/vb_core/src/frame.rs:1638:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 120: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.103
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1643:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 121: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.104
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1645:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 122: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.105
	 - Status: SUCCESS
	 - Description: "X->S!"
	 - Location: crates/vb_core/src/frame.rs:1646:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 123: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.106
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1651:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 124: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.107
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1653:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 125: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.108
	 - Status: SUCCESS
	 - Description: "X->F!"
	 - Location: crates/vb_core/src/frame.rs:1654:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 126: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.109
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1659:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 127: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.110
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1661:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 128: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.111
	 - Status: SUCCESS
	 - Description: "X->W!"
	 - Location: crates/vb_core/src/frame.rs:1662:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 129: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.112
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1667:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 130: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.113
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1669:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 131: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.114
	 - Status: SUCCESS
	 - Description: "X->A!"
	 - Location: crates/vb_core/src/frame.rs:1670:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 132: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.115
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1675:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 133: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.116
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1677:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 134: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.117
	 - Status: SUCCESS
	 - Description: "X->C!"
	 - Location: crates/vb_core/src/frame.rs:1678:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 135: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.118
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1683:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 136: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.119
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1685:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 137: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.120
	 - Status: SUCCESS
	 - Description: "X->X"
	 - Location: crates/vb_core/src/frame.rs:1686:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 138: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.121
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1694:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 139: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.122
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1696:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 140: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.123
	 - Status: SUCCESS
	 - Description: "W->P!"
	 - Location: crates/vb_core/src/frame.rs:1697:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 141: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.124
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1702:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 142: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.125
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1704:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 143: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.126
	 - Status: SUCCESS
	 - Description: "W->R"
	 - Location: crates/vb_core/src/frame.rs:1705:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 144: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.127
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1710:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 145: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.128
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1712:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 146: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.129
	 - Status: SUCCESS
	 - Description: "W->S!"
	 - Location: crates/vb_core/src/frame.rs:1713:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 147: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.130
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1718:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 148: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.131
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1720:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 149: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.132
	 - Status: SUCCESS
	 - Description: "W->F!"
	 - Location: crates/vb_core/src/frame.rs:1721:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 150: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.133
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1726:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 151: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.134
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1728:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 152: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.135
	 - Status: SUCCESS
	 - Description: "W->K!"
	 - Location: crates/vb_core/src/frame.rs:1729:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 153: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.136
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1734:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 154: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.137
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1736:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 155: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.138
	 - Status: SUCCESS
	 - Description: "W->W"
	 - Location: crates/vb_core/src/frame.rs:1737:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 156: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.139
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1742:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 157: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.140
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1744:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 158: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.141
	 - Status: SUCCESS
	 - Description: "W->A!"
	 - Location: crates/vb_core/src/frame.rs:1745:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 159: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.142
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1750:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 160: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.143
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1752:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 161: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.144
	 - Status: SUCCESS
	 - Description: "W->C!"
	 - Location: crates/vb_core/src/frame.rs:1753:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 162: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.145
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1761:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 163: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.146
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1763:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 164: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.147
	 - Status: SUCCESS
	 - Description: "A->P!"
	 - Location: crates/vb_core/src/frame.rs:1764:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 165: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.148
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1769:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 166: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.149
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1771:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 167: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.150
	 - Status: SUCCESS
	 - Description: "A->R"
	 - Location: crates/vb_core/src/frame.rs:1772:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 168: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.151
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1777:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 169: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.152
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1779:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 170: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.153
	 - Status: SUCCESS
	 - Description: "A->S!"
	 - Location: crates/vb_core/src/frame.rs:1780:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 171: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.154
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1785:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 172: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.155
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1787:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 173: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.156
	 - Status: SUCCESS
	 - Description: "A->F!"
	 - Location: crates/vb_core/src/frame.rs:1788:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 174: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.157
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1793:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 175: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.158
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1795:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 176: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.159
	 - Status: SUCCESS
	 - Description: "A->K!"
	 - Location: crates/vb_core/src/frame.rs:1796:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 177: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.160
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1801:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 178: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.161
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1803:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 179: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.162
	 - Status: SUCCESS
	 - Description: "A->W!"
	 - Location: crates/vb_core/src/frame.rs:1804:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 180: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.163
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1809:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 181: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.164
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1811:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 182: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.165
	 - Status: SUCCESS
	 - Description: "A->A"
	 - Location: crates/vb_core/src/frame.rs:1812:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 183: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.166
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1817:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 184: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.167
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1819:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 185: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.168
	 - Status: SUCCESS
	 - Description: "A->C!"
	 - Location: crates/vb_core/src/frame.rs:1820:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 186: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.169
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1828:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 187: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.170
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1830:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 188: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.171
	 - Status: SUCCESS
	 - Description: "!->P!"
	 - Location: crates/vb_core/src/frame.rs:1831:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 189: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.172
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1836:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 190: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.173
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1838:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 191: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.174
	 - Status: SUCCESS
	 - Description: "!->R!"
	 - Location: crates/vb_core/src/frame.rs:1839:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 192: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.175
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1844:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 193: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.176
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1846:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 194: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.177
	 - Status: SUCCESS
	 - Description: "!->S!"
	 - Location: crates/vb_core/src/frame.rs:1847:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 195: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.178
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1852:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 196: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.179
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1854:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 197: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.180
	 - Status: SUCCESS
	 - Description: "!->F!"
	 - Location: crates/vb_core/src/frame.rs:1855:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 198: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.181
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1860:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 199: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.182
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1862:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 200: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.183
	 - Status: SUCCESS
	 - Description: "!->K!"
	 - Location: crates/vb_core/src/frame.rs:1863:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 201: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.184
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1868:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 202: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.185
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1870:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 203: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.186
	 - Status: SUCCESS
	 - Description: "!->W!"
	 - Location: crates/vb_core/src/frame.rs:1871:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 204: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.187
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1876:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 205: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.188
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1878:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 206: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.189
	 - Status: SUCCESS
	 - Description: "!->A!"
	 - Location: crates/vb_core/src/frame.rs:1879:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 207: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.190
	 - Status: UNREACHABLE
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1884:21 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 208: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.191
	 - Status: SUCCESS
	 - Description: "attempt to add with overflow"
	 - Location: crates/vb_core/src/frame.rs:1886:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 209: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.192
	 - Status: SUCCESS
	 - Description: "!-->!"
	 - Location: crates/vb_core/src/frame.rs:1887:17 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 210: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.193
	 - Status: SUCCESS
	 - Description: "exhaustive 64 pairs covered"
	 - Location: crates/vb_core/src/frame.rs:1891:9 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 211: frame::frame_kani_harnesses::validate_transition_exhaustive_64.assertion.194
	 - Status: SUCCESS
	 - Description: "all 64 pairs validated correctly"
	 - Location: crates/vb_core/src/frame.rs:1892:9 in function frame::frame_kani_harnesses::validate_transition_exhaustive_64

Check 212: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.1
	 - Status: SUCCESS
	 - Description: "R->R"
	 - Location: crates/vb_core/src/frame.rs:1918:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 213: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.2
	 - Status: SUCCESS
	 - Description: "R->S"
	 - Location: crates/vb_core/src/frame.rs:1919:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 214: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.3
	 - Status: SUCCESS
	 - Description: "R->F"
	 - Location: crates/vb_core/src/frame.rs:1920:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 215: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.4
	 - Status: SUCCESS
	 - Description: "R->W"
	 - Location: crates/vb_core/src/frame.rs:1921:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 216: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.5
	 - Status: SUCCESS
	 - Description: "R->A"
	 - Location: crates/vb_core/src/frame.rs:1922:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 217: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.6
	 - Status: SUCCESS
	 - Description: "R->C"
	 - Location: crates/vb_core/src/frame.rs:1923:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 218: frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets.assertion.7
	 - Status: SUCCESS
	 - Description: "R->K"
	 - Location: crates/vb_core/src/frame.rs:1924:9 in function frame::frame_kani_harnesses::validate_transition_running_to_all_valid_targets

Check 219: kani::rustc_intrinsics::offset::<frame::StepState, *mut frame::StepState, usize>.safety_check.1
	 - Status: SUCCESS
	 - Description: "Offset in bytes overflows isize"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function kani::rustc_intrinsics::offset::<frame::StepState, *mut frame::StepState, usize>

Check 220: kani::rustc_intrinsics::offset::<frame::StepState, *mut frame::StepState, usize>.safety_check.2
	 - Status: SUCCESS
	 - Description: "Offset result and original pointer must point to the same allocation"
	 - Location: ../../../../runner/work/kani/kani/library/kani/src/lib.rs:57:1 in function kani::rustc_intrinsics::offset::<frame::StepState, *mut frame::StepState, usize>

Check 221: frame::frame_kani_harnesses::validate_transition_idempotent.assertion.1
	 - Status: SUCCESS
	 - Description: "attempt to calculate the remainder with a divisor of zero"
	 - Location: crates/vb_core/src/frame.rs:1909:40 in function frame::frame_kani_harnesses::validate_transition_idempotent

Check 222: frame::frame_kani_harnesses::validate_transition_idempotent.arithmetic_overflow.1
	 - Status: SUCCESS
	 - Description: "attempt to calculate the remainder with a divisor of zero"
	 - Location: crates/vb_core/src/frame.rs:1909:40 in function frame::frame_kani_harnesses::validate_transition_idempotent

Check 223: frame::frame_kani_harnesses::validate_transition_idempotent.assertion.2
	 - Status: SUCCESS
	 - Description: "self-transition always valid"
	 - Location: crates/vb_core/src/frame.rs:1911:9 in function frame::frame_kani_harnesses::validate_transition_idempotent

Check 224: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.1
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:9 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 225: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.2
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:9 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 226: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.3
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:9 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 227: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.4
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:9 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 228: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.5
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:9 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 229: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.6
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:9 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 230: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.7
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:26 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 231: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.8
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:26 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 232: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.9
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:26 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 233: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.10
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:26 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 234: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.11
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:26 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 235: <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq.pointer_dereference.12
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:1692:26 in function <std::ptr::NonNull<frame::StepState> as std::cmp::PartialEq>::eq

Check 236: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.pointer_dereference.1
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:20 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 237: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.pointer_dereference.2
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:20 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 238: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.pointer_dereference.3
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:20 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 239: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.pointer_dereference.4
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:20 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 240: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.pointer_dereference.5
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:20 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 241: std::ptr::NonNull::<frame::StepState>::as_ref::<'_>.pointer_dereference.6
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/non_null.rs:445:20 in function std::ptr::NonNull::<frame::StepState>::as_ref::<'_>

Check 242: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.1
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:160:27 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 243: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.2
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:160:27 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 244: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.3
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:160:27 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 245: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.4
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:160:27 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 246: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.5
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:160:27 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 247: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.6
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:160:27 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 248: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.7
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:161:34 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 249: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.8
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:161:34 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 250: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.9
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:161:34 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 251: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.10
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:161:34 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 252: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.11
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:161:34 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 253: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.12
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:161:34 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 254: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.13
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:174:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 255: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.14
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:174:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 256: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.15
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:174:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 257: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.16
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:174:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 258: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.17
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:174:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 259: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.18
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:174:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 260: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.19
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:168:36 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 261: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.20
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:180:36 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 262: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.21
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:184:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 263: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.22
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:184:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 264: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.23
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:184:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 265: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.24
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:184:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 266: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.25
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:184:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 267: <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next.pointer_dereference.26
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: ../../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:184:25 in function <std::slice::Iter<'_, frame::StepState> as std::iter::Iterator>::next

Check 268: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.1
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: crates/vb_core/src/frame.rs:1946:26 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 269: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.2
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: crates/vb_core/src/frame.rs:1946:14 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 270: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.3
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: crates/vb_core/src/frame.rs:1946:14 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 271: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.4
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: crates/vb_core/src/frame.rs:1946:14 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 272: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.5
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: crates/vb_core/src/frame.rs:1946:14 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 273: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.6
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: crates/vb_core/src/frame.rs:1946:14 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 274: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.7
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: crates/vb_core/src/frame.rs:1946:14 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 275: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.8
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: crates/vb_core/src/frame.rs:1947:28 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 276: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.9
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: crates/vb_core/src/frame.rs:1947:18 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 277: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.10
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: crates/vb_core/src/frame.rs:1947:18 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 278: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.11
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: crates/vb_core/src/frame.rs:1947:18 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 279: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.12
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: crates/vb_core/src/frame.rs:1947:18 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 280: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.13
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: crates/vb_core/src/frame.rs:1947:18 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 281: frame::frame_kani_harnesses::validate_transition_terminal_blocks_all.pointer_dereference.14
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: crates/vb_core/src/frame.rs:1947:18 in function frame::frame_kani_harnesses::validate_transition_terminal_blocks_all

Check 282: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.1
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 283: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.2
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 284: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.3
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 285: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.4
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 286: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.5
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 287: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.6
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 288: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.7
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer NULL"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 289: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.8
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer invalid"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 290: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.9
	 - Status: SUCCESS
	 - Description: "dereference failure: deallocated dynamic object"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 291: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.10
	 - Status: SUCCESS
	 - Description: "dereference failure: dead object"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 292: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.11
	 - Status: SUCCESS
	 - Description: "dereference failure: pointer outside object bounds"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq

Check 293: <frame::StepState as std::cmp::PartialEq>::eq.pointer_dereference.12
	 - Status: SUCCESS
	 - Description: "dereference failure: invalid integer address"
	 - Location: crates/vb_core/src/frame.rs:10:30 in function <frame::StepState as std::cmp::PartialEq>::eq


SUMMARY:
 ** 0 of 293 failed (65 unreachable)

VERIFICATION:- SUCCESSFUL
Verification Time: 3.9564145s

Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.

COMMAND=TMPDIR=/home/lewis/src/tmp_build/vb-qi37.4.2-kani CARGO_TARGET_DIR=/home/lewis/src/tmp_build/vb-qi37.4.2-cargo-target SCCACHE_DIR=/home/lewis/src/tmp_build/vb-qi37.4.2-sccache SCCACHE_TMPDIR=/home/lewis/src/tmp_build/vb-qi37.4.2-kani RUSTC_WRAPPER= cargo kani -p vb_core --harness kani_step_state
EXIT_STATUS=0
