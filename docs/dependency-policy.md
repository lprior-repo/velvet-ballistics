# Dependency Policy

First-party code is zero-unsafe. Third-party crates may use internal `unsafe` only when they are explicitly justified, maintained, and mechanically audited.

## Initial Runtime Dependencies

| Crate | Reason |
| --- | --- |
| `fjall` | embedded append-only durability substrate |
| `postcard` | compact stable binary event encoding |
| `bytes` | cheap shared byte payloads at IPC/action boundaries |
| `crossbeam-channel` | bounded high-throughput memory ingress |
| `serde` | stable data encoding derives |
| `thiserror` | typed errors without stringly failure paths |
| `arrayvec` | fixed-capacity stack key construction |
| `blake3` | fast compiled-workflow digest placeholder |
| `saphyr` | native Rust YAML AST parser for cold strict validation |
| `serde-saphyr` | typed YAML deserialization layer after strict pre-validation |
| `tempfile` | isolated storage tests without handwritten temp paths |

## Review Questions For New Dependencies

1. What handwritten infrastructure does this remove?
2. Does it add runtime allocation, blocking, or task spawning?
3. Does it use `unsafe`, and is that acceptable for this layer?
4. Can malformed input cause panic?
5. Are default features disabled where practical?
6. Is it maintained and compatible with nightly April 2026 policy?
7. Does it pass audit, deny, vet, and geiger gates?
8. Is it allowed in the crate layer that wants to use it?
