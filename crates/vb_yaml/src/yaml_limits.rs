#![forbid(unsafe_code)]

//! Strict YAML profile limits.

/// Strict YAML profile limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    /// Maximum source text size in bytes.
    pub max_source_bytes: usize,
    /// Maximum nesting depth.
    pub max_depth: u16,
    /// Maximum total YAML nodes visited.
    pub max_nodes: u32,
    /// Maximum sequence length.
    pub max_sequence_len: usize,
    /// Maximum mapping entry count.
    pub max_mapping_entries: usize,
    /// Maximum scalar value length in bytes.
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 1_048_576,
            max_depth: 64,
            max_nodes: 100_000,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        }
    }
}
