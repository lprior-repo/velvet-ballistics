use super::*;
use bytes::Bytes;
use std::num::NonZeroUsize;
use vb_core::{RunId, WorkflowDigest};

#[test]
fn ingress_frame_new_with_empty_payload_and_min_max_succeeds() {
    let min_max = MaxPayloadBytes::new(NonZeroUsize::MIN);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::new(),
        min_max,
    );
    let expected = IngressFrame {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        payload: crate::BoundedPayload::new(Bytes::new(), min_max).unwrap(),
    };
    assert_eq!(result, Ok(expected));
}

#[test]
fn ingress_frame_new_with_payload_exactly_at_max_succeeds() {
    let max = MaxPayloadBytes::DEFAULT;
    let payload = Bytes::from(vec![0u8; max.get()]);
    let result = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        payload.clone(),
        max,
    );
    let expected = IngressFrame {
        run_id: RunId::new(1),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        payload: crate::BoundedPayload::new(payload, max).unwrap(),
    };
    assert_eq!(result, Ok(expected));
}

#[test]
fn memory_ingress_bounded_capacity_one_accepts_one_rejects_second() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::new(),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    assert_eq!(ingress.try_submit(frame.clone()), Ok(()));
    assert!(matches!(ingress.try_submit(frame), Err(IpcError::Full)));
}

#[test]
fn memory_ingress_try_recv_on_empty_queue_returns_none() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    assert_eq!(ingress.try_recv(), Ok(None));
}

#[test]
fn memory_ingress_try_recv_returns_items_in_fifo_order() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(2).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let frame1 = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"first"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    let frame2 = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"second"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    ingress.try_submit(frame1.clone()).unwrap();
    ingress.try_submit(frame2.clone()).unwrap();

    assert_eq!(ingress.try_recv(), Ok(Some(frame1)));
    assert_eq!(ingress.try_recv(), Ok(Some(frame2)));
    assert_eq!(ingress.try_recv(), Ok(None));
}

#[test]
fn memory_ingress_producer_handle_preserves_queued_frames() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(2).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let producer = ingress.producer();
    let first = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"first"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    let second = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"second"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    assert_eq!(ingress.try_submit(first.clone()), Ok(()));
    assert_eq!(producer.try_submit(second.clone()), Ok(()));

    assert_eq!(ingress.try_recv(), Ok(Some(first)));
    assert_eq!(ingress.try_recv(), Ok(Some(second)));
    assert_eq!(ingress.try_recv(), Ok(None));
}

#[test]
fn cloned_producer_handles_share_queue_backpressure() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let first_producer = ingress.producer();
    let second_producer = first_producer.clone();
    let first = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"first"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();
    let second = IngressFrame::new(
        RunId::new(2),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"second"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    assert_eq!(first_producer.try_submit(first.clone()), Ok(()));
    assert_eq!(
        second_producer.try_submit(second.clone()),
        Err(IpcError::Full)
    );
    assert_eq!(ingress.try_recv(), Ok(Some(first)));
    assert_eq!(second_producer.try_submit(second.clone()), Ok(()));
    assert_eq!(ingress.try_recv(), Ok(Some(second)));
}

#[test]
fn producer_handle_try_submit_returns_disconnected_after_receiver_drop() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let ingress = MemoryIngress::bounded(capacity);
    let producer = ingress.producer();
    let frame = IngressFrame::new(
        RunId::new(1),
        WorkflowDigest::from_bytes([0u8; 32]),
        Bytes::from_static(b"frame"),
        MaxPayloadBytes::DEFAULT,
    )
    .unwrap();

    drop(ingress);

    assert_eq!(producer.try_submit(frame), Err(IpcError::Disconnected));
}

#[test]
fn memory_ingress_recv_returns_disconnected_after_sender_drop() {
    let capacity = QueueCapacity::new(NonZeroUsize::new(1).unwrap());
    let mut ingress = MemoryIngress::bounded(capacity);
    ingress.disconnect_sender();
    assert!(matches!(ingress.try_recv(), Err(IpcError::Disconnected)));
}
