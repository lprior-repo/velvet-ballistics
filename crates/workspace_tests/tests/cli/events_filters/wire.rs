#![cfg(test)]

use super::support::TestResult;

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
