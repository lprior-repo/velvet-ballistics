use super::support::{Fixture, TestResult};
use vb_core::value_store::transport::CheckpointWireCursor as Cursor;

pub(super) type Positions = Vec<(&'static str, usize)>;

fn mark(cursor: &Cursor<'_>, size: usize, key: &'static str, fields: &mut Positions) {
    fields.push((key, size.saturating_sub(cursor.remaining().len())));
}

fn count(
    cursor: &mut Cursor<'_>,
    size: usize,
    key: &'static str,
    fields: &mut Positions,
) -> TestResult<usize> {
    mark(cursor, size, key, fields);
    Ok(usize::try_from(cursor.unsigned(64)?)?)
}

pub(super) fn positions(bytes: &[u8], fixture: &Fixture) -> TestResult<Positions> {
    let size = bytes.len();
    let mut c = Cursor::new(bytes);
    let mut fields = Vec::new();
    c.unsigned(16)?;
    c.take(postcard::to_allocvec(&fixture.expected)?.len())?;
    c.take(64)?;
    c.unsigned(32)?;
    for _ in 0..3 {
        c.unsigned(16)?;
    }
    frame(&mut c, size, &mut fields)?;
    arenas(&mut c, size, &mut fields)?;
    for _ in 0..count(&mut c, size, "action_attempts", &mut fields)? {
        c.unsigned(16)?;
    }
    let admission = fixture
        .state
        .admission
        .as_ref()
        .ok_or("admission missing")?;
    let admission_bytes = postcard::to_allocvec(admission)?;
    mark(&c, size, "admission", &mut fields);
    let mut admission_cursor = Cursor::new(c.remaining());
    admission_cursor.take(32)?;
    admission_cursor.unsigned(64)?;
    mark(&admission_cursor, size, "admission_grants", &mut fields);
    c.take(admission_bytes.len())?;
    count(&mut c, size, "collect_entries", &mut fields)?;
    count(&mut c, size, "collect_lineages", &mut fields)?;
    mark(&c, size, "action_contracts", &mut fields);
    contracts(&mut c, size, &mut fields)?;
    assert_eq!(c.remaining().len(), 32);
    Ok(fields)
}

fn frame(c: &mut Cursor<'_>, size: usize, fields: &mut Positions) -> TestResult {
    c.unsigned(64)?;
    c.unsigned(16)?;
    c.unsigned(64)?;
    c.unsigned(16)?;
    c.unsigned(16)?;
    for _ in 0..count(c, size, "frame_states", fields)? {
        c.unsigned(32)?;
    }
    for _ in 0..count(c, size, "frame_slots", fields)? {
        match c.take(1)?.first().copied() {
            Some(1) => c.value()?,
            Some(0) => {}
            _ => return Err("bad genuine option".into()),
        }
    }
    for _ in 0..count(c, size, "frame_taints", fields)? {
        c.taint()?;
    }
    Ok(())
}

fn arenas(c: &mut Cursor<'_>, size: usize, fields: &mut Positions) -> TestResult {
    for _ in 0..count(c, size, "symbols", fields)? {
        let len = count(c, size, "symbol_bytes", fields)?;
        c.take(len)?;
    }
    for _ in 0..count(c, size, "lists", fields)? {
        for _ in 0..count(c, size, "list_items", fields)? {
            c.value()?;
        }
    }
    for _ in 0..count(c, size, "list_taints", fields)? {
        for _ in 0..count(c, size, "item_taints", fields)? {
            c.taint()?;
        }
    }
    for _ in 0..count(c, size, "objects", fields)? {
        for _ in 0..count(c, size, "object_fields", fields)? {
            c.unsigned(32)?;
            c.value()?;
            c.taint()?;
        }
    }
    for _ in 0..count(c, size, "blobs", fields)? {
        let len = count(c, size, "blob_bytes", fields)?;
        c.take(len)?;
    }
    c.unsigned(64)?;
    Ok(())
}

pub(super) fn huge_length(bytes: &[u8], offset: usize) -> TestResult<Vec<u8>> {
    let mut cursor = Cursor::new(bytes.get(offset..).ok_or("offset missing")?);
    cursor.unsigned(64)?;
    let end = bytes
        .len()
        .checked_sub(cursor.remaining().len())
        .ok_or("offset overflow")?;
    let mut result = bytes.get(..offset).ok_or("prefix missing")?.to_vec();
    result.extend(postcard::to_allocvec(&u64::MAX)?);
    result.extend_from_slice(bytes.get(end..).ok_or("suffix missing")?);
    Ok(result)
}

fn contracts(c: &mut Cursor<'_>, size: usize, fields: &mut Positions) -> TestResult {
    let number = c.unsigned(64)?;
    assert_eq!(number, 1);
    c.unsigned(16)?;
    let name_len = count(c, size, "contract_name", fields)?;
    c.take(name_len)?;
    c.unsigned(16)?;
    c.unsigned(16)?;
    c.unsigned(32)?;
    c.unsigned(32)?;
    c.unsigned(64)?;
    for _ in 0..3 {
        c.unsigned(32)?;
    }
    let caps = count(c, size, "contract_capabilities", fields)?;
    assert_eq!(caps, 0);
    Ok(())
}
