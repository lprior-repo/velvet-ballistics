# Formal Verification Report

Status: PASS
Generated: 2026-05-10T15:34:06Z
Bead: vb-nf2u

## Moon verification lanes

- verify-fast: PASS (executed through verify-standard/deep/all).
- verify-standard: PASS (executed through verify-deep/all).
- verify-deep: PASS.
- verify-proof: PASS.
- verify-all: PASS.

## Five verification lanes

- Kani: formal proof (Kani inventory + layout harnesses).
- Miri: undefined behavior (miri test).
- Lockbud: concurrency (waived by WAIVE-CONCURRENCY-UI-RELEASE for vb-nf2u).
- fuzz: coverage (cargo fuzz smoke).
- coverage: llvm-cov nextest.

## Kani persisted summaries

- Kani inventory summary: `.evidence/vb-nf2u/kani-ui.txt` present and non-empty.
  ```text
17:Checking harness kani_harnesses::inventory_exactly_matches_canonical_screens...
20:Reading GOTO program from file /home/lewis/src/Velvet-ballistics-vb-nf2u-go/target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_ui_snapshot-ef944768bb703dc6__RNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screens.out
45:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:5:49: 5:57}> thread 0
70:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss0_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:6:49: 6:57}> thread 0
71:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss0_0EB1I_.0 iteration 2 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:6:49: 6:57}> thread 0
89:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss1_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}> thread 0
90:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss1_0EB1I_.0 iteration 2 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}> thread 0
91:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss1_0EB1I_.0 iteration 3 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}> thread 0
116:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss2_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}> thread 0
117:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss2_0EB1I_.0 iteration 2 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}> thread 0
118:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss2_0EB1I_.0 iteration 3 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}> thread 0
119:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss2_0EB1I_.0 iteration 4 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}> thread 0
134:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss3_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}> thread 0
135:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss3_0EB1I_.0 iteration 2 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}> thread 0
136:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss3_0EB1I_.0 iteration 3 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}> thread 0
137:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss3_0EB1I_.0 iteration 4 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}> thread 0
138:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss3_0EB1I_.0 iteration 5 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}> thread 0
155:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss4_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}> thread 0
156:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss4_0EB1I_.0 iteration 2 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}> thread 0
157:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss4_0EB1I_.0 iteration 3 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}> thread 0
158:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss4_0EB1I_.0 iteration 4 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}> thread 0
159:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss4_0EB1I_.0 iteration 5 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}> thread 0
160:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss4_0EB1I_.0 iteration 6 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}> thread 0
176:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 1 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
177:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 2 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
178:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 3 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
179:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 4 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
180:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 5 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
181:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 6 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
182:Unwinding loop _RINvXs2E_NtNtCsci0VKyKEi6N_4core5slice4iterINtB7_4IterReENtNtNtNtBb_4iter6traits8iterator8Iterator3anyNCNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses43inventory_exactly_matches_canonical_screenss5_0EB1I_.0 iteration 7 file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs line 307 column 17 function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}> thread 0
257:Check 6: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.1
260:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:3:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
262:Check 7: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.2
265:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:4:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
267:Check 8: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.3
271:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:5:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
273:Check 9: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.4
276:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:6:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
278:Check 10: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.5
282:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:7:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
284:Check 11: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.6
287:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:8:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
289:Check 12: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.7
292:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:9:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
294:Check 13: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.8
297:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:10:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
299:Check 14: kani_harnesses::inventory_exactly_matches_canonical_screens.assertion.9
303:	 - Location: crates/vb_ui_snapshot/src/../kani/inventory.rs:11:5 in function kani_harnesses::inventory_exactly_matches_canonical_screens
305:Check 15: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:6:49: 6:57}>.unreachable.1
308:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:6:49: 6:57}>
310:Check 16: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}>.unreachable.1
313:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}>
330:Check 20: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}>.unreachable.1
333:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}>
365:Check 27: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:5:49: 5:57}>.unreachable.1
368:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:5:49: 5:57}>
370:Check 28: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:4:49: 4:57}>.unreachable.1
373:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:4:49: 4:57}>
375:Check 29: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}>.unreachable.1
378:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}>
380:Check 30: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}>.unreachable.1
383:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}>
410:Check 36: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}>.unreachable.1
413:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:302:13 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}>
415:Check 37: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}>.pointer_dereference.1
418:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:8:49: 8:57}>
480:Check 50: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}>.pointer_dereference.1
483:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:7:49: 7:57}>
485:Check 51: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:6:49: 6:57}>.pointer_dereference.1
488:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:6:49: 6:57}>
640:Check 82: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}>.pointer_dereference.1
643:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:10:49: 10:57}>
775:Check 109: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:5:49: 5:57}>.pointer_dereference.1
778:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:5:49: 5:57}>
780:Check 110: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:4:49: 4:57}>.pointer_dereference.1
783:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:4:49: 4:57}>
785:Check 111: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}>.pointer_dereference.1
788:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:9:49: 9:57}>
790:Check 112: <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}>.pointer_dereference.1
793:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/slice/iter/macros.rs:307:37 in function <std::slice::Iter<'_, &str> as std::iter::Iterator>::any::<{closure@crates/vb_ui_snapshot/src/../kani/inventory.rs:11:49: 11:57}>
856:SUMMARY:
863:Complete - 1 successfully verified harnesses, 0 failures, 1 total.
  ```
- Kani layout summary: `.evidence/vb-nf2u/kani-layout.txt` present and non-empty.
  ```text
10:Checking harness kani_harnesses::layout_selected_state_requires_visible_positive_indicator...
13:Reading GOTO program from file /home/lewis/src/Velvet-ballistics-vb-nf2u-go/target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_ui_snapshot-2d0c4f0793524e6f__RNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses57layout_selected_state_requires_visible_positive_indicator.out
20:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
21:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
22:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
23:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
24:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::rect_contains thread 0
25:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 835 column 15 function std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:137:39: 137:50}> thread 0
26:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
27:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
28:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
29:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
30:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
31:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::rect_contains thread 0
32:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
63:Check 1: std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>.unreachable.1
66:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1338:15 in function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>
68:Check 2: std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:137:39: 137:50}>.unreachable.1
71:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:835:15 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:137:39: 137:50}>
73:Check 3: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.unreachable.1
76:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
78:Check 4: kani_harnesses::layout_selected_state_requires_visible_positive_indicator.assertion.1
81:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:79:5 in function kani_harnesses::layout_selected_state_requires_visible_positive_indicator
83:Check 5: kani_harnesses::layout_selected_state_requires_visible_positive_indicator.assertion.2
86:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:81:9 in function kani_harnesses::layout_selected_state_requires_visible_positive_indicator
88:Check 6: kani_harnesses::layout_selected_state_requires_visible_positive_indicator.assertion.3
91:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:82:9 in function kani_harnesses::layout_selected_state_requires_visible_positive_indicator
93:Check 7: kani_harnesses::layout_selected_state_requires_visible_positive_indicator.assertion.4
96:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:83:9 in function kani_harnesses::layout_selected_state_requires_visible_positive_indicator
103:Check 9: layout_kernel::selected_state_is_visible.unreachable.1
106:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:132:22 in function layout_kernel::selected_state_is_visible
108:Check 10: layout_kernel::rect_contains.unreachable.1
111:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function layout_kernel::rect_contains
113:Check 11: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch.unreachable.1
116:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:2173:15 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch
118:Check 12: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.1
121:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
123:Check 13: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.2
126:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
128:Check 14: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.3
131:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
133:Check 15: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.4
136:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
138:Check 16: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.5
141:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
143:Check 17: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.6
146:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
148:Check 18: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.7
151:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
153:Check 19: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.8
156:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
158:Check 20: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.9
161:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
163:Check 21: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.10
166:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
168:Check 22: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.11
171:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
173:Check 23: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.12
176:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
178:Check 24: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.1
181:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
183:Check 25: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.2
186:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
188:Check 26: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.3
191:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
193:Check 27: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.4
196:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
198:Check 28: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.5
201:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
203:Check 29: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.6
206:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
208:Check 30: layout_kernel::selected_state_is_visible::{closure#0}.pointer_dereference.1
211:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:137:87 in function layout_kernel::selected_state_is_visible::{closure#0}
213:Check 31: layout_kernel::selected_state_is_visible::{closure#0}.pointer_dereference.2
216:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:137:87 in function layout_kernel::selected_state_is_visible::{closure#0}
218:Check 32: layout_kernel::selected_state_is_visible::{closure#0}.pointer_dereference.3
221:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:137:87 in function layout_kernel::selected_state_is_visible::{closure#0}
223:Check 33: layout_kernel::selected_state_is_visible::{closure#0}.pointer_dereference.4
226:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:137:87 in function layout_kernel::selected_state_is_visible::{closure#0}
228:Check 34: layout_kernel::selected_state_is_visible::{closure#0}.pointer_dereference.5
231:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:137:87 in function layout_kernel::selected_state_is_visible::{closure#0}
233:Check 35: layout_kernel::selected_state_is_visible::{closure#0}.pointer_dereference.6
236:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:137:87 in function layout_kernel::selected_state_is_visible::{closure#0}
238:Check 36: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.1
241:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
243:Check 37: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.2
246:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
248:Check 38: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.3
251:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
253:Check 39: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.4
256:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
258:Check 40: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.5
261:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
263:Check 41: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.6
266:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
268:Check 42: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.7
271:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
273:Check 43: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.8
276:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
278:Check 44: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.9
281:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
283:Check 45: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.10
286:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
288:Check 46: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.11
291:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
293:Check 47: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.12
296:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
358:Check 60: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.1
361:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
363:Check 61: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.2
366:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
368:Check 62: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.3
371:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
373:Check 63: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.4
376:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
378:Check 64: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.5
381:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
383:Check 65: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.6
386:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
388:Check 66: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.7
391:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
393:Check 67: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.8
396:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
398:Check 68: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.9
401:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
403:Check 69: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.10
406:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
408:Check 70: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.11
411:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
413:Check 71: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.12
416:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
418:Check 72: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.13
421:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
423:Check 73: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.14
426:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
428:Check 74: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.15
431:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
433:Check 75: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.16
436:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
438:Check 76: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.17
441:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
443:Check 77: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.18
446:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
448:Check 78: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.19
451:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
453:Check 79: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.20
456:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
458:Check 80: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.21
461:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
463:Check 81: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.22
466:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
468:Check 82: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.23
471:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
473:Check 83: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.24
476:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
478:Check 84: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.25
481:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
483:Check 85: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.26
486:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
488:Check 86: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.27
491:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
493:Check 87: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.28
496:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
498:Check 88: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.29
501:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
503:Check 89: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.30
506:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
569:SUMMARY:
575:Checking harness kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast...
578:Reading GOTO program from file /home/lewis/src/Velvet-ballistics-vb-nf2u-go/target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_ui_snapshot-2d0c4f0793524e6f__RNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses61layout_chip_readability_requires_area_dimensions_and_contrast.out
609:Check 1: kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast.assertion.1
612:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:68:9 in function kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast
614:Check 2: kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast.assertion.2
617:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:69:9 in function kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast
619:Check 3: kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast.assertion.3
622:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:70:9 in function kani_harnesses::layout_chip_readability_requires_area_dimensions_and_contrast
625:SUMMARY:
631:Checking harness kani_harnesses::layout_bounds_rejects_controls_outside_viewport...
634:Reading GOTO program from file /home/lewis/src/Velvet-ballistics-vb-nf2u-go/target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_ui_snapshot-2d0c4f0793524e6f__RNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses47layout_bounds_rejects_controls_outside_viewport.out
641:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
642:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
643:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
644:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
645:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::rect_contains thread 0
646:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
647:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
648:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
649:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
650:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::rect_contains thread 0
651:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 835 column 15 function std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:118:42: 118:53}> thread 0
652:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
653:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
684:Check 1: std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:118:42: 118:53}>.unreachable.1
687:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:835:15 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:118:42: 118:53}>
689:Check 2: layout_kernel::rect_contains.unreachable.1
692:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function layout_kernel::rect_contains
694:Check 3: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch.unreachable.1
697:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:2173:15 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch
704:Check 5: kani_harnesses::layout_bounds_rejects_controls_outside_viewport.assertion.1
707:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:54:5 in function kani_harnesses::layout_bounds_rejects_controls_outside_viewport
709:Check 6: kani_harnesses::layout_bounds_rejects_controls_outside_viewport.assertion.2
712:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:55:5 in function kani_harnesses::layout_bounds_rejects_controls_outside_viewport
714:Check 7: kani_harnesses::layout_bounds_rejects_controls_outside_viewport.assertion.3
717:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:57:9 in function kani_harnesses::layout_bounds_rejects_controls_outside_viewport
719:Check 8: std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>.unreachable.1
722:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1338:15 in function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>
724:Check 9: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.unreachable.1
727:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
789:Check 22: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.1
792:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
794:Check 23: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.2
797:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
799:Check 24: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.3
802:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
804:Check 25: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.4
807:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
809:Check 26: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.5
812:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
814:Check 27: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.6
817:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
819:Check 28: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.7
822:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
824:Check 29: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.8
827:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
829:Check 30: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.9
832:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
834:Check 31: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.10
837:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
839:Check 32: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.11
842:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
844:Check 33: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.12
847:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
849:Check 34: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.1
852:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
854:Check 35: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.2
857:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
859:Check 36: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.3
862:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
864:Check 37: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.4
867:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
869:Check 38: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.5
872:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
874:Check 39: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.6
877:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
879:Check 40: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.1
882:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
884:Check 41: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.2
887:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
889:Check 42: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.3
892:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
894:Check 43: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.4
897:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
899:Check 44: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.5
902:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
904:Check 45: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.6
907:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
909:Check 46: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.7
912:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
914:Check 47: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.8
917:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
919:Check 48: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.9
922:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
924:Check 49: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.10
927:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
929:Check 50: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.11
932:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
934:Check 51: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.12
937:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
999:Check 64: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.1
1002:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1004:Check 65: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.2
1007:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1009:Check 66: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.3
1012:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1014:Check 67: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.4
1017:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1019:Check 68: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.5
1022:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1024:Check 69: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.6
1027:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1029:Check 70: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.7
1032:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1034:Check 71: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.8
1037:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1039:Check 72: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.9
1042:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1044:Check 73: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.10
1047:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1049:Check 74: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.11
1052:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1054:Check 75: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.12
1057:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1059:Check 76: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.13
1062:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1064:Check 77: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.14
1067:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1069:Check 78: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.15
1072:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1074:Check 79: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.16
1077:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1079:Check 80: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.17
1082:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1084:Check 81: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.18
1087:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1089:Check 82: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.19
1092:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1094:Check 83: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.20
1097:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1099:Check 84: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.21
1102:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1104:Check 85: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.22
1107:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1109:Check 86: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.23
1112:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1114:Check 87: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.24
1117:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1119:Check 88: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.25
1122:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1124:Check 89: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.26
1127:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1129:Check 90: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.27
1132:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1134:Check 91: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.28
1137:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1139:Check 92: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.29
1142:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1144:Check 93: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.30
1147:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1150:SUMMARY:
1156:Checking harness kani_harnesses::layout_clipping_rejects_rectangles_outside_container...
1159:Reading GOTO program from file /home/lewis/src/Velvet-ballistics-vb-nf2u-go/target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_ui_snapshot-2d0c4f0793524e6f__RNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses52layout_clipping_rejects_rectangles_outside_container.out
1166:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1167:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1168:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1169:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1170:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::rect_contains thread 0
1171:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1172:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1173:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1174:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1175:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::rect_contains thread 0
1176:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 835 column 15 function std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:114:41: 114:52}> thread 0
1177:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
1178:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
1209:Check 1: kani_harnesses::layout_clipping_rejects_rectangles_outside_container.assertion.1
1212:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:41:5 in function kani_harnesses::layout_clipping_rejects_rectangles_outside_container
1214:Check 2: kani_harnesses::layout_clipping_rejects_rectangles_outside_container.assertion.2
1217:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:42:5 in function kani_harnesses::layout_clipping_rejects_rectangles_outside_container
1219:Check 3: kani_harnesses::layout_clipping_rejects_rectangles_outside_container.assertion.3
1222:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:44:9 in function kani_harnesses::layout_clipping_rejects_rectangles_outside_container
1224:Check 4: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.unreachable.1
1227:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1229:Check 5: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch.unreachable.1
1232:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:2173:15 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch
1234:Check 6: std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>.unreachable.1
1237:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1338:15 in function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>
1244:Check 8: std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:114:41: 114:52}>.unreachable.1
1247:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:835:15 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::map::<bool, {closure@crates/vb_ui_snapshot/src/layout_kernel.rs:114:41: 114:52}>
1249:Check 9: layout_kernel::rect_contains.unreachable.1
1252:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function layout_kernel::rect_contains
1254:Check 10: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.1
1257:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1259:Check 11: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.2
1262:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1264:Check 12: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.3
1267:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1269:Check 13: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.4
1272:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1274:Check 14: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.5
1277:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1279:Check 15: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.6
1282:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1284:Check 16: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.7
1287:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1289:Check 17: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.8
1292:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1294:Check 18: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.9
1297:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1299:Check 19: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.10
1302:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1304:Check 20: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.11
1307:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1309:Check 21: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.12
1312:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1314:Check 22: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.1
1317:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
1319:Check 23: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.2
1322:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
1324:Check 24: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.3
1327:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
1329:Check 25: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.4
1332:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
1334:Check 26: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.5
1337:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
1339:Check 27: std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok.pointer_dereference.6
1342:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:594:18 in function std::result::Result::<bool, layout_kernel::LayoutKernelError>::is_ok
1404:Check 40: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.1
1407:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1409:Check 41: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.2
1412:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1414:Check 42: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.3
1417:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1419:Check 43: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.4
1422:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1424:Check 44: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.5
1427:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1429:Check 45: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.6
1432:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1434:Check 46: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.7
1437:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1439:Check 47: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.8
1442:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1444:Check 48: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.9
1447:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1449:Check 49: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.10
1452:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1454:Check 50: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.11
1457:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1459:Check 51: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.12
1462:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1464:Check 52: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.13
1467:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1469:Check 53: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.14
1472:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1474:Check 54: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.15
1477:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1479:Check 55: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.16
1482:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1484:Check 56: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.17
1487:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1489:Check 57: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.18
1492:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1494:Check 58: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.19
1497:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1499:Check 59: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.20
1502:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1504:Check 60: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.21
1507:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1509:Check 61: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.22
1512:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1514:Check 62: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.23
1517:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1519:Check 63: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.24
1522:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1524:Check 64: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.25
1527:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1529:Check 65: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.26
1532:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1534:Check 66: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.27
1537:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1539:Check 67: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.28
1542:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1544:Check 68: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.29
1547:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1549:Check 69: <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.30
1552:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<bool, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1554:Check 70: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.1
1557:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1559:Check 71: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.2
1562:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1564:Check 72: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.3
1567:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1569:Check 73: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.4
1572:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1574:Check 74: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.5
1577:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1579:Check 75: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.6
1582:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1584:Check 76: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.7
1587:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1589:Check 77: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.8
1592:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1594:Check 78: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.9
1597:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1599:Check 79: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.10
1602:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1604:Check 80: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.11
1607:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1609:Check 81: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.12
1612:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
1675:SUMMARY:
1681:Checking harness kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked...
1684:Reading GOTO program from file /home/lewis/src/Velvet-ballistics-vb-nf2u-go/target/kani/x86_64-unknown-linux-gnu/debug/deps/vb_ui_snapshot-2d0c4f0793524e6f__RNvNtCsdlaIdwnkl3C_14vb_ui_snapshot14kani_harnesses49layout_overlap_predicate_is_symmetric_and_checked.out
1691:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1692:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1693:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1694:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1695:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1696:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1697:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1698:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1699:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1700:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1701:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1702:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1703:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::overlap_area_px thread 0
1704:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1705:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1706:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1707:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1708:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1709:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1710:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1711:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1712:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1713:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1714:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1715:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1716:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs line 2173 column 15 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch thread 0
1717:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function layout_kernel::overlap_area_px thread 0
1718:aborting path on assume(false) at file /home/runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs line 1338 column 15 function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError> thread 0
1719:aborting path on assume(false) at file crates/vb_ui_snapshot/src/lib.rs line 0 column 0 function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq thread 0
1720:aborting path on assume(false) at file crates/vb_ui_snapshot/src/../kani/layout_predicates.rs line 29 column 11 function kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked thread 0
1750:Check 2: std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>.unreachable.1
1753:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/option.rs:1338:15 in function std::option::Option::<u32>::ok_or::<layout_kernel::LayoutKernelError>
1755:Check 3: layout_kernel::overlap_area_px.unreachable.1
1758:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function layout_kernel::overlap_area_px
1765:Check 5: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch.unreachable.1
1768:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:2173:15 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::ops::Try>::branch
1770:Check 6: kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked.assertion.1
1773:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:28:5 in function kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked
1775:Check 7: kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked.unreachable.1
1778:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:29:11 in function kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked
1780:Check 8: kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked.assertion.2
1783:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:31:19 in function kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked
1785:Check 9: kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked.assertion.3
1788:	 - Location: crates/vb_ui_snapshot/src/../kani/layout_predicates.rs:30:21 in function kani_harnesses::layout_overlap_predicate_is_symmetric_and_checked
1790:Check 10: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.unreachable.1
1793:	 - Location: crates/vb_ui_snapshot/src/lib.rs:0:0 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1795:Check 11: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.1
1798:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1800:Check 12: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.2
1803:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1805:Check 13: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.3
1808:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1810:Check 14: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.4
1813:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1815:Check 15: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.5
1818:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1820:Check 16: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.6
1823:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1825:Check 17: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.7
1828:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1830:Check 18: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.8
1833:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1835:Check 19: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.9
1838:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1840:Check 20: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.10
1843:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1845:Check 21: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.11
1848:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1850:Check 22: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.12
1853:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1855:Check 23: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.13
1858:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1860:Check 24: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.14
1863:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1865:Check 25: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.15
1868:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1870:Check 26: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.16
1873:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1875:Check 27: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.17
1878:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1880:Check 28: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.18
1883:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1885:Check 29: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.19
1888:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1890:Check 30: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.20
1893:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1895:Check 31: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.21
1898:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1900:Check 32: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.22
1903:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1905:Check 33: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.23
1908:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1910:Check 34: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.24
1913:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1915:Check 35: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.25
1918:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1920:Check 36: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.26
1923:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1925:Check 37: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.27
1928:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1930:Check 38: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.28
1933:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1935:Check 39: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.29
1938:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1940:Check 40: <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq.pointer_dereference.30
1943:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/result.rs:553:16 in function <std::result::Result<u32, layout_kernel::LayoutKernelError> as std::cmp::PartialEq>::eq
1945:Check 41: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.1
1948:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1950:Check 42: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.2
1953:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1955:Check 43: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.3
1958:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1960:Check 44: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.4
1963:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1965:Check 45: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.5
1968:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1970:Check 46: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.6
1973:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1975:Check 47: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.7
1978:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1980:Check 48: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.8
1983:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1985:Check 49: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.9
1988:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1990:Check 50: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.10
1993:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
1995:Check 51: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.11
1998:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
2000:Check 52: <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq.pointer_dereference.12
2003:	 - Location: crates/vb_ui_snapshot/src/layout_kernel.rs:11:30 in function <layout_kernel::LayoutKernelError as std::cmp::PartialEq>::eq
2125:Check 77: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.1
2128:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2130:Check 78: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.2
2133:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2135:Check 79: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.3
2138:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2140:Check 80: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.4
2143:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2145:Check 81: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.5
2148:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2150:Check 82: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.6
2153:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:27 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2155:Check 83: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.7
2158:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2160:Check 84: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.8
2163:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2165:Check 85: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.9
2168:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2170:Check 86: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.10
2173:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2175:Check 87: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.11
2178:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2180:Check 88: std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq.pointer_dereference.12
2183:	 - Location: ../../../runner/.rustup/toolchains/nightly-2025-11-21-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/cmp.rs:2090:34 in function std::cmp::impls::<impl std::cmp::PartialEq for &layout_kernel::LayoutKernelError>::eq
2246:SUMMARY:
2253:Complete - 5 successfully verified harnesses, 0 failures, 5 total.
  ```

## Miri evidence

- Miri: `moon run :verify-deep` runs miri test as part of deep verification.
- Lane status: PASS when `moon run :verify-all` completes without miri failure.

## Coverage evidence

- Coverage: `moon run :verify-deep` runs `moon run :coverage` as part of deep verification.
- Lane status: PASS when `moon run :verify-all` completes without coverage failure.

## Lockbud waiver evidence

- Lockbud waived only by bead-scoped `WAIVE-CONCURRENCY-UI-RELEASE` artifact.
- VERIFY_BEAD_ID: `vb-nf2u`.
- ALLOW_BEAD_LOCKBUD_WAIVER: `1`.
- Waiver validation: PASS when `moon run :verify-all` reaches this report.
