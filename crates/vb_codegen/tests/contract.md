# Contract: `trybuild_compile_fail_tests` / `trybuild_pass_tests`

## Function Signature

```rust
fn trybuild_compile_fail_tests() -> Result<(), String>
fn trybuild_pass_tests() -> Result<(), String>
```

Both functions return `Result<(), String>` where the `String` is a human-readable error message.

## Error Variants (as String messages)

### `NoCompileFailFixturesFound`

Returned when the `compile-fail/` directory exists but contains no `.rs` files.

```rust
Err("No compile-fail fixtures found in {path}")
```

### `CompileFailDirectoryNotReadable`

Returned when the `compile-fail/` directory cannot be read (permission denied, I/O error, etc.).

```rust
Err("...error message from fs::read_dir...")
```

### `NoPassFixturesFound`

Returned when the `pass/` directory exists but contains no `.rs` files.

```rust
Err("No pass fixtures found in {path}")
```

### `PassDirectoryNotFound`

Returned when the `pass/` directory does not exist.

```rust
Err("No pass fixtures directory found at {path}")
```

### `PassDirectoryNotReadable`

Returned when the `pass/` directory exists but cannot be read.

```rust
Err("...error message from fs::read_dir...")
```

## Pure Helper: `filter_compile_fail_fixtures`

```rust
fn filter_compile_fail_fixtures(dir: &Path) -> Result<Vec<PathBuf>, String>
```

Reads a directory and returns all `.rs` files.

- **Ok(vec)**: Directory read successfully; may be empty
- **Err(String)**: Directory could not be read (I/O error)

## Postconditions

- `trybuild_compile_fail_tests` returns `Ok(())` **only** when trybuild confirms at least one fixture produced the expected compile error. It does NOT return `Ok(())` for empty directories.
- `trybuild_pass_tests` returns `Ok(())` **only** when trybuild confirms all pass fixtures compile without error.
- Empty directory is a failure condition, NOT a silent pass.
