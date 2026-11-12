# CCC IPC fixture port

Compose frozen candidate ae767b77c2d054b9d5da010bc79e4c27587123d3 and upstream ccc958d784849f46b335ba416e2d9a3c28c6ca0a from common base 65549576afe7066b53edcdd7a17d1ece29e307ed in this unique colocated Git/JJ workspace.

Resolve only crates/vb_ipc/src/server/impl_tests.rs by retaining the complete upstream root imports, remaining helpers, and seven include declarations in order, replacing its temp_socket_path definition with the exact candidate definition and appending the exact candidate ipc_socket_path_uses_configured_temp_dir test after the includes.

The helper must honor inherited TMPDIR through std::env::temp_dir(), retain the existing atomic uniqueness counter and eight-character basename truncation, and preserve the regression assertions for the configured parent and Unix socket path length below 108 bytes. No root /tmp access is authorized.

Keep every upstream include chunk and every other upstream changed path byte-identical to CCC where disjoint; inherited protected configuration removals are upstream bytes only and receive no manual edits. Preserve all candidate changes outside the incoming path set. No test names, assertions, production APIs, schemas, dependencies, gates, proofs, or configuration are authored or relaxed by this port.

Compute a complete upstream path/blob preservation ledger, exact helper/test byte hashes and unaffected-root byte guard, include-order guard, and case-name inventory. Freeze source in a new empty JJ child before delivery; do not commit, push, close the bead, or mutate other workspaces.

Run only vb_ipc server::impl_tests scoped nextest cases in default and all-features modes with pinned nightly-2026-04-28, jobs two, disabled wrappers, RUSTFLAGS=-Dwarnings, TMPDIR=/run/user/1000/vbs, and an owned target directory under a 24 GiB memory cap and five-minute timeout per invocation. Record exact argv, source pin, logs, exit or signal, and any timeout or setup failure without substituting recalled evidence. File-only rustfmt must use edition/style-edition 2024 and skip_children=true to protect the upstream include chunks.

Independent CODE and TEST reviews and parent canonical CI remain separate requirements; scoped native results do not establish phase, product, or full workspace acceptance. No Kani or other solver launch is part of this task.
