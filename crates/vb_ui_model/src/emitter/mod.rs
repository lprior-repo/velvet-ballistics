//! CLI text and binary envelope emitters.

mod binary;
mod error;
mod yaml;

pub use binary::decode_postcard;
pub use binary::encode_postcard;
pub use error::EmitterError;
pub use yaml::{TEXT_SCHEMA_VERSION, YamlEnvelope, encode_yaml};
