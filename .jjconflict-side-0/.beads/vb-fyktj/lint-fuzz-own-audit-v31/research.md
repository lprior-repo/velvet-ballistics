# Own-Branch Historical Audit — vb-fyktj

Audit scope: three historical own-branch commits compared against current main source. No shell execution. No broad verification. Per-hunk preservation analysis only.

## Patch 1: a46c3723dc — lint as_mut_slice (3 files)

### Hunk 1.1: crates/vb_ipc/src/frame_types.rs
Old: `let mut cursor = std::io::Cursor::new(&mut bytes[..]);`
Current (line 41): `let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());`
Verdict: preserved_exact
Evidence: identical as_mut_slice() call at same logical location in encode() method.

### Hunk 1.2: crates/workspace_tests/src/test_util/fixture.rs
Old: `rng.fill(&mut vec[..]);`
Current (line 58): `rng.fill(vec.as_mut_slice());`
Verdict: preserved_exact
Evidence: identical as_mut_slice() call in build_bytes() method.

### Hunk 1.3: crates/workspace_tests/src/test_util/seed.rs
Old: `rng.fill(&mut bytes[..]);`
Current (line 23): `rng.fill(bytes.as_mut_slice());`
Verdict: preserved_exact
Evidence: identical as_mut_slice() call in SeededBytes::new() method.

## Patch 2: 62a2f47c2f — fuzz payload bound and profile comments

### Hunk 2.1: fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs
Old: Added doc comment with `-max_len=65536 -runs=100000` command hint.
Current (lines 1-3): Doc comment preserved exactly.
Verdict: preserved_exact

### Hunk 2.2: fuzz/fuzz_targets/yaml_events.rs
Old: Added doc comment with `-max_len=65536 -runs=100000` command hint.
Current (lines 1-7): Doc comment preserved exactly.
Verdict: preserved_exact

### Hunk 2.3: fuzz/src/yaml_target.rs
Old: `const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;` → `100_000`
Current (line 17): `const MAX_FUZZ_PAYLOAD_USIZE: usize = 100_000;`
Verdict: preserved_exact
Evidence: constant value matches; no subsequent regression detected.

## Patch 3: c56283c963 — corpora and dictionary smoke changes

### Hunk 3.1: fuzz corpus loading
Old: Added `if [ -d "fuzz/corpus/${fuzz_target}" ]; then cp -R ...`
Current (lines 578-580): Corpus loading preserved.
Verdict: preserved_exact

### Hunk 3.2: fuzz dictionary loading
Old: `dict_flag=""` shell variable approach with `split`-style positional arg.
Current (lines 581-586): Dictionary loading preserved but using bash array `fuzz_args+=()` with `-dict=` syntax.
Verdict: superseded_valid
Evidence: intent and behavior preserved; syntax modernized to array-based arg building. 8 .dict files exist (journal_event, kind_validation, vb_f04l_yaml_compiler_compile, vb_storage_codec, decode_record, expression, ipc_frame, journal_decode).

### Hunk 3.3: fuzz/corpus input dependency
Old: Added `'fuzz/corpus/**/*'` to inputs array.
Current (line 599): `'fuzz/corpus/**/*'` preserved in inputs.
Verdict: preserved_exact

## Overall

All 9 hunks across 3 patches have been preserved in current source. One hunk (dict loading syntax) was modernized to array-based syntax but maintains identical behavior. No missing, obsolete, or unsafe hunks detected. No inherited_unresolved from these three own-patches.

Context: Current main has newer fuzz child with actual 93 targets and 5 matching dictionaries. Earlier rawall8 dictionary parser probes also present. This audit covers only the three named own-patches; full v1 closure is not claimed.
