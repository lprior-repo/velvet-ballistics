// ══ Proptest module ════════════════════════════════════════════════════════════

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn regression_config() -> ProptestConfig {
        ProptestConfig {
            failure_persistence: Some(Box::new(
                proptest::test_runner::FileFailurePersistence::Direct(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/proptest-regressions/tests.txt"
                )),
            )),
            ..ProptestConfig::default()
        }
    }

    proptest! {
        #![proptest_config(regression_config())]
        #[test]
        fn ipc_command_roundtrips_through_u16(cmd in 1u16..=11u16) {
            let parsed = IpcCommand::from_u16(cmd);
            prop_assert_ok!(parsed);
            let Ok(command) = parsed else { return Ok(()) };
            prop_assert_eq!(command.as_u16(), cmd);
        }
    }

    proptest! {
        #![proptest_config(regression_config())]
        #[test]
        fn non_magic_bytes_always_rejected(magic in 0u32..) {
            if magic != IPC_MAGIC {
                let mut header_bytes = [0u8; IPC_HEADER_LEN];
                header_bytes[..4].copy_from_slice(&magic.to_le_bytes());
                header_bytes[4..6].copy_from_slice(&IPC_VERSION.to_le_bytes());
                let result = IpcFrameHeader::decode(&header_bytes, MaxPayloadBytes::DEFAULT);
                prop_assert_err!(result);
                if let Err(e) = result {
                    prop_assert!(
                        matches!(e, IpcError::InvalidMagic { .. }),
                        "expected InvalidMagic, got {e:?}"
                    );
                }
            }
        }
    }

    proptest! {
        #![proptest_config(regression_config())]
        #[test]
        fn ipc_command_encode_decode_roundtrip(cmd_val in 1u16..=11u16) {
            let Ok(command) = IpcCommand::from_u16(cmd_val) else {
                return Ok(());
            };

            let header = IpcFrameHeader::new(command, 0, 0, 0);
            let encoded = header.encode();
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = IpcFrameHeader::decode(&encoded, MaxPayloadBytes::DEFAULT);

            prop_assert_ok!(decoded);
            let Ok(decoded) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded.command, command);
        }
    }

    proptest! {
        #![proptest_config(regression_config())]
        #[test]
        fn ipc_response_encode_decode_roundtrip(run_id_val in 0u64..) {
            let payload = IpcPayload::CancelRun { run_id: RunId::new(run_id_val) };

            let encoded = encode_payload(&payload, MaxPayloadBytes::DEFAULT);
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            let decoded = decode_payload(&encoded);

            prop_assert_ok!(decoded);
            let Ok(decoded) = decoded else { return Ok(()) };
            prop_assert_eq!(decoded, payload);
        }
    }

    proptest! {
        #![proptest_config(regression_config())]
        #[test]
        fn frame_header_length_never_exceeds_max(cmd_val in 1u16..=11u16, payload_len in 0u32..=1024u32) {
            let Ok(command) = IpcCommand::from_u16(cmd_val) else {
                return Ok(());
            };

            let header = IpcFrameHeader::new(command, 0, 0, payload_len);

            let encoded = header.encode();
            prop_assert_ok!(encoded);
            let Ok(encoded) = encoded else { return Ok(()) };
            prop_assert_eq!(encoded.len(), IPC_HEADER_LEN);
        }
    }
}
