#![forbid(unsafe_code)]
#[cfg(test)]
mod tests {
    use crate::JournalError;
    use crate::keys::KeyspaceScanPolicy;
    use crate::preview::preview_keyspace;
    use crate::types::PreviewConfig;
    #[test]
    fn empty_entries_produces_empty_preview() {
        let config = PreviewConfig::new(10, 1024).unwrap();
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![];
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        assert!(result.entries.is_empty());
        assert!(!result.truncated);
        assert_eq!(result.total_keyspace_records, 0);
    }

    #[test]
    fn max_records_cap_never_exceeded() {
        let config = PreviewConfig::new(3, 1024 * 1024).unwrap();
        // Create 10 entries but cap at 3.
        let entries: Vec<_> = (0..10)
            .map(|_| (vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1], vec![42u8; 10]))
            .collect();
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        assert!(result.entries.len() <= 3);
        assert!(result.truncated);
    }

    #[test]
    fn max_bytes_hard_cap_never_exceeded() {
        let config = PreviewConfig::new(100, 50).unwrap();
        let entries: Vec<_> = (0..10)
            .map(|_| (vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1], vec![0u8; 20]))
            .collect();
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        // Each entry is 20 bytes, max_bytes is 50. At most 2 entries (40 bytes) +
        // the 3rd would be 60 which exceeds 50, so max 2 entries.
        assert!(result.entries.len() <= 5);
        let total: u32 = result.entries.iter().map(|(_, v, _)| v.len() as u32).sum();
        assert!(total <= 50);
    }

    // -----------------------------------------------------------------
    // CC-002 follow-up: policy-aware malformed-key handling.
    // -----------------------------------------------------------------

    #[test]
    fn preview_keyspace_skips_malformed() {
        // SkipMalformed: a corrupt key in the middle of an entry list
        // is silently dropped; surrounding valid entries still appear.
        let config = PreviewConfig::new(10, 1024).unwrap();
        // Valid run-header keys for runs 1, 2, 3.
        let make_valid = |run: u64| {
            crate::keys::run_header_key(vb_core::RunId::new(run))
                .unwrap()
                .to_vec()
        };
        // A length-mismatched run-header key: PREFIX_RUN_HEADER + 2 bytes
        // (expected 9 bytes total). Structural `KeyLengthMismatch` error.
        let short_key = vec![0x10, 0xAB, 0xCD];
        let entries = vec![
            (make_valid(1), vec![0xAAu8; 4]),
            (short_key, vec![0xBBu8; 4]),
            (make_valid(2), vec![0xCCu8; 4]),
            (make_valid(3), vec![0xDDu8; 4]),
        ];
        let mut scratch: Vec<u8> = Vec::new();
        let result = preview_keyspace(
            KeyspaceScanPolicy::default_doctor(),
            config,
            &entries,
            &mut scratch,
        )
        .unwrap();
        assert_eq!(result.entries.len(), 3);
        assert!(!result.truncated);
        assert_eq!(result.total_keyspace_records, 4);
        // The three surviving entries are runs 1, 2, 3 — order preserved.
        for (i, (key, _, _)) in result.entries.iter().enumerate() {
            match key {
                crate::StorageKey::RunHeader { run } => {
                    assert_eq!(run.get(), (i as u64) + 1);
                }
                other => panic!("expected RunHeader, got {other:?}"),
            }
        }
    }

    #[test]
    fn preview_keyspace_fails_closed() {
        // FailClosed: a malformed key aborts the scan and surfaces a
        // typed `MalformedKeyspaceRow` error carrying prefix, expected
        // length, and actual length.
        let config = PreviewConfig::new(10, 1024).unwrap();
        let make_valid = |run: u64| {
            crate::keys::run_header_key(vb_core::RunId::new(run))
                .unwrap()
                .to_vec()
        };
        // Well-formed entry first so the policy reaches a malformed row
        // at index 1 (the length-mismatch case).
        let short_key: Vec<u8> = vec![0x10, 0x00, 0x00, 0x00]; // 4 bytes, expected 9
        let entries = vec![
            (make_valid(1), vec![0xAAu8; 4]),
            (short_key.clone(), vec![0xBBu8; 4]),
            (make_valid(2), vec![0xCCu8; 4]),
        ];
        let mut scratch: Vec<u8> = Vec::new();
        let err = preview_keyspace(
            KeyspaceScanPolicy::default_production(),
            config,
            &entries,
            &mut scratch,
        )
        .expect_err("FailClosed must abort on malformed key");
        match err {
            JournalError::MalformedKeyspaceRow {
                prefix,
                expected_len,
                actual_len,
            } => {
                assert_eq!(prefix, 0x10);
                assert_eq!(expected_len, 9);
                assert_eq!(actual_len, short_key.len());
            }
            other => panic!("expected MalformedKeyspaceRow, got {other:?}"),
        }
    }

    #[test]
    fn preview_keyspace_fail_closed_unknown_prefix() {
        // FailClosed path also catches unknown-prefix keys with the
        // typed error. `expected_len` is 0 because there is no
        // structural expectation for an unrecognised prefix byte.
        let config = PreviewConfig::new(10, 1024).unwrap();
        // First byte 0xFF is not one of the nine known prefixes.
        let entries: Vec<(Vec<u8>, Vec<u8>)> = vec![(vec![0xFF, 0x01, 0x02, 0x03], vec![0u8; 4])];
        let mut scratch: Vec<u8> = Vec::new();
        let err = preview_keyspace(
            KeyspaceScanPolicy::default_production(),
            config,
            &entries,
            &mut scratch,
        )
        .expect_err("FailClosed must abort on unknown prefix");
        match err {
            JournalError::MalformedKeyspaceRow {
                prefix,
                expected_len,
                actual_len,
            } => {
                assert_eq!(prefix, 0xFF);
                assert_eq!(expected_len, 0);
                assert_eq!(actual_len, 4);
            }
            other => panic!("expected MalformedKeyspaceRow, got {other:?}"),
        }
    }
}
