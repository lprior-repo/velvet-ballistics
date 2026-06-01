//! Unit tests for [`IpcActionOutputPayload`].

use vb_ipc::action_output::IpcActionOutputPayload;
use vb_core::ids::SlotIdx;
use vb_core::value::{SlotValue, Taint};

fn sample_payload() -> IpcActionOutputPayload {
    IpcActionOutputPayload {
        output_slot: SlotIdx::new(3),
        value: SlotValue::I64(42),
        taint: Taint::Clean,
    }
}

#[test]
fn into_action_output_preserves_output_slot() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(100);
    assert_eq!(action_output.output_slot, SlotIdx::new(3));
}

#[test]
fn into_action_output_preserves_value() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(100);
    assert_eq!(action_output.value, SlotValue::I64(42));
}

#[test]
fn into_action_output_preserves_taint() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(100);
    assert_eq!(action_output.taint, Taint::Clean);
}

#[test]
fn into_action_output_stores_encoded_len() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(256);
    assert_eq!(action_output.encoded_len, 256);
}

#[test]
fn into_action_output_with_zero_encoded_len() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let action_output = payload.into_action_output(0);
    assert_eq!(action_output.encoded_len, 0);
    assert_eq!(action_output.output_slot, SlotIdx::ZERO);
}

#[test]
fn into_action_output_with_max_encoded_len() {
    let payload = sample_payload();
    let action_output = payload.into_action_output(u32::MAX);
    assert_eq!(action_output.encoded_len, u32::MAX);
}

#[test]
fn into_action_output_with_secret_taint() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Bool(true),
        taint: Taint::Secret,
    };
    let action_output = payload.into_action_output(10);
    assert_eq!(action_output.taint, Taint::Secret);
}

#[test]
fn into_action_output_with_derived_from_secret_taint() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::DerivedFromSecret,
    };
    let action_output = payload.into_action_output(5);
    assert_eq!(action_output.taint, Taint::DerivedFromSecret);
}

#[test]
fn postcard_roundtrip_with_null_value() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::ZERO,
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let encoded = postcard::to_allocvec(&payload);
    let Ok(encoded) = encoded else {
        assert!(false, "postcard encoding should succeed");
        return;
    };
    let decoded: Result<IpcActionOutputPayload, _> = postcard::from_bytes(&encoded);
    let Ok(decoded) = decoded else {
        assert!(false, "postcard decoding should succeed");
        return;
    };
    assert_eq!(decoded.output_slot, payload.output_slot);
    assert_eq!(decoded.value, payload.value);
    assert_eq!(decoded.taint, payload.taint);
}

#[test]
fn postcard_roundtrip_with_bool_value() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(5),
        value: SlotValue::Bool(false),
        taint: Taint::Clean,
    };
    let Ok(encoded) = postcard::to_allocvec(&payload) else {
        return;
    };
    let decoded: IpcActionOutputPayload = match postcard::from_bytes(&encoded) {
        Ok(d) => d,
        Err(_) => {
            assert!(false, "decoding should succeed");
            return;
        }
    };
    assert_eq!(decoded.value, SlotValue::Bool(false));
}

#[test]
fn postcard_roundtrip_with_i64_value() {
    let payload = IpcActionOutputPayload {
        output_slot: SlotIdx::new(2),
        value: SlotValue::I64(-100),
        taint: Taint::DerivedFromSecret,
    };
    let Ok(encoded) = postcard::to_allocvec(&payload) else {
        return;
    };
    let decoded: IpcActionOutputPayload = match postcard::from_bytes(&encoded) {
        Ok(d) => d,
        Err(_) => {
            assert!(false, "decoding should succeed");
            return;
        }
    };
    assert_eq!(decoded.value, SlotValue::I64(-100));
    assert_eq!(decoded.taint, Taint::DerivedFromSecret);
}

#[test]
fn ipc_action_output_payload_equality() {
    let a = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let b = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    assert_eq!(a, b);
}

#[test]
fn ipc_action_output_payload_inequality_different_slot() {
    let a = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let b = IpcActionOutputPayload {
        output_slot: SlotIdx::new(2),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    assert_ne!(a, b);
}

#[test]
fn ipc_action_output_payload_inequality_different_taint() {
    let a = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Clean,
    };
    let b = IpcActionOutputPayload {
        output_slot: SlotIdx::new(1),
        value: SlotValue::Null,
        taint: Taint::Secret,
    };
    assert_ne!(a, b);
}
