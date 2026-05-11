#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let expected_magic = vb_storage::MAGIC_JOURNAL_EVENT;
    let max_payload_len = 1024u32;

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record_header(data, expected_magic, max_payload_len);

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        expected_magic,
        max_payload_len,
    );

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_BLOB,
        max_payload_len,
    );

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_COMPILED_ARTIFACT,
        max_payload_len,
    );

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_SNAPSHOT,
        max_payload_len,
    );

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_WORKFLOW_SOURCE,
        max_payload_len,
    );

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_INDEX_RECORD,
        max_payload_len,
    );

    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(data, 0xFFFF_FFFF, max_payload_len);
    #[allow(clippy::let_underscore_must_use)]
    let _ = vb_storage::decode_record::<vb_storage::JournalEvent>(data, 0x0000_0000, max_payload_len);
});
