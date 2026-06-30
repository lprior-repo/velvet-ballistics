---
section: 31
title: "Mandatory Function Surface: `vb_ipc`"
parent: velvet-ballistics-MASTER.md
---

## 31. Mandatory Function Surface: `vb_ipc`


**Source of truth:** `crates/vb_ipc/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Frame encode/decode | `encode_frame`, `decode_frame_header`, `decode_frame_payload`, `validate_frame_bounds`. |
| Server | `serve_ipc` (mio-based Unix socket loop, all 11 command handlers). |
| Client | `IpcClient::connect`, `send_command`, `recv_response`. |
| Command handlers | `handle_submit_run`, `handle_submit_run_inline`, `handle_cancel_run`, `handle_inspect_run`, `handle_list_events`, `handle_answer_ask`, `handle_complete_action`, `handle_fail_action`, `handle_drain_trace`, `handle_health`, `handle_shutdown`. |

---
