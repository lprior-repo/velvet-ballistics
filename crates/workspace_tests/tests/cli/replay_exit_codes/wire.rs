#![cfg(test)]

use super::support::TestResult;
use std::process::Output;

pub(super) fn decode(bytes: &[u8], emit: &str) -> TestResult<serde_json::Value> {
    if emit == "yaml" {
        return Ok(serde_saphyr::from_str(std::str::from_utf8(bytes)?)?);
    }
    let header = bytes.get(..52).ok_or("CLI header")?;
    assert_eq!(header.get(..4), Some(b"VCLA".as_slice()));
    assert_eq!(header.get(4..8), Some([1, 0, 2, 0].as_slice()));
    assert_eq!(header.get(8..12), Some(52u32.to_le_bytes().as_slice()));
    let payload = bytes.get(52..).ok_or("CLI payload")?;
    assert_eq!(
        header.get(12..16),
        Some(u32::try_from(payload.len())?.to_le_bytes().as_slice())
    );
    assert_eq!(
        header.get(16..48),
        Some(blake3::hash(payload).as_bytes().as_slice())
    );
    let crc = crc32fast::hash(header.get(..48).ok_or("CRC input")?);
    assert_eq!(header.get(48..52), Some(crc.to_le_bytes().as_slice()));
    let (version, kind, content_type, json): (u16, u16, u32, Vec<u8>) =
        postcard::from_bytes(payload)?;
    assert_eq!((version, kind, content_type), (1, 2, 0));
    Ok(serde_json::from_slice(&json)?)
}

pub(super) fn assert_open_error(output: Output, emit: &str, message: &str) -> TestResult {
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    if emit == "text" {
        assert_eq!(String::from_utf8(output.stderr)?, format!("{message}\n"));
    } else {
        assert_eq!(
            decode(&output.stderr, emit)?,
            serde_json::json!({
                "success": false, "error": "StorageError", "exit_code": 5, "message": message
            })
        );
    }
    Ok(())
}
