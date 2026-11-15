# Reject short frame boundaries

The public IPC extract_payload helper must return existing IpcServerError::IncompleteFrame for any total_len below IPC_HEADER_LEN, before splitting or replacing the caller buffer. Inputs with insufficient available bytes keep the same error and nonmutation. A complete header with zero payload, valid payload extraction and unread following-frame bytes retain their current behavior. Both public helper aliases and the reactor caller use this shared implementation.

No signature, error variant, dependency, gate, wire/persisted format or runtime authority change. Baseline native test must expose the actual short-frame panic; final regression covers every sub-header boundary with empty, partial and populated buffers and exact whole-buffer preservation. Existing IPC framing and partial-write tests must remain and run; independent CODE and TEST plus canonical joined checks precede landing.
