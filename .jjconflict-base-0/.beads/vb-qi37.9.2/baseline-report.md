   Compiling vb_runtime v0.1.0 (/home/lewis/src/crates/vb_runtime)
warning: ignoring -C extra-filename flag due to -o flag

error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory (os error 2)
 --> crates/vb_runtime/src/runtime.rs:4:1
  |
4 | include!("runtime/chunk_001.rs");
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: `vb_runtime` (lib) generated 1 warning
error: could not compile `vb_runtime` (lib) due to 1 previous error; 1 warning emitted
