#![forbid(unsafe_code)]

use vb_core::WorkflowDigest;
use vb_storage::{BlobRecord, FjallJournal, JournalError};

#[test]
fn blob_point_reads_and_trim_preserve_other_digest_payloads()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let journal = FjallJournal::open(directory.path(), None)?;
    let records: Vec<_> = [
        b"first payload".as_slice(),
        b"second payload",
        b"third payload",
    ]
    .into_iter()
    .map(|bytes| BlobRecord {
        digest: blake3::hash(bytes).into(),
        bytes: bytes.to_vec(),
    })
    .collect();
    for record in &records {
        journal.put_blob(record)?;
    }
    for record in &records {
        assert_eq!(journal.blob(record.digest)?, Some(record.clone()));
    }
    let target = records.get(1).ok_or("missing target fixture")?;
    journal.trim_blob(target.digest)?;
    assert_eq!(journal.blob(target.digest)?, None);
    let missing = *blake3::hash(b"never inserted").as_bytes();
    for digest in [target.digest, missing] {
        assert!(
            matches!(journal.trim_blob(digest), Err(JournalError::ArtifactNotFound { digest: actual }) if actual == WorkflowDigest::from_bytes(digest))
        );
        assert_eq!(journal.blob(digest)?, None);
        for record in records
            .iter()
            .filter(|record| record.digest != target.digest)
        {
            assert_eq!(journal.blob(record.digest)?, Some(record.clone()));
        }
    }
    Ok(())
}
