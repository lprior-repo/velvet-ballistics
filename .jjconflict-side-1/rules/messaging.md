# Events, queues, and webhooks

Read this when building async messaging: message queues, event streams, pub/sub, or inbound/outbound webhooks.

- Never publish to a broker inside the request path or transaction: write the domain change and an outbox row in the same DB transaction, then a separate relay reads the outbox and publishes. The commit is the single source of truth — the message can't exist without the state change or vice versa.
- The relay drains the outbox with `SELECT ... FOR UPDATE SKIP LOCKED` plus a `dispatched` flag, so multiple worker instances poll concurrently without double-sending.
- Delivery is at-least-once, so consumers must be idempotent: carry a stable event id, dedupe on it (unique constraint or seen-set), and make the handler safe to run twice. Derive queue job ids deterministically (`endpointId:eventId`) so a re-enqueue can't double-deliver.
- Consumer retry policy: bounded attempts, exponential backoff with jitter, throw to trigger retry; after max attempts move the message to a dead-letter queue and record why.
- Persist one row per delivery attempt for visibility.
- A consumer retry budget is not an outbound HTTP retry budget — the broker already redelivers, so counting both without noticing multiplies the attempts (`rules/backend-security.md` for the outbound numbers).
- Classify a consumer failure before retrying it: timeouts, 5xx responses and lock contention are retriable; validation failures and business-rule rejections are terminal and go straight to the dead-letter queue with the reason, instead of spending the retry budget on a message that can never succeed.
- An event is a past-tense fact that any number of consumers, or none, may handle (`OrderPlaced`); a command asks one owner to act (`SendInvoice`) and needs that owner to exist and succeed. Decide which one a message is before naming it: a command broadcast as an event has nothing enforcing that anyone handled it.
- Inbound webhooks you receive: verify the signature against the raw request body before parsing, reject timestamps outside a tolerance window (replay defense), and respond fast — do the work async. Return 2xx only once the event is durably accepted; return 5xx to make the sender retry (`rules/payments.md` for the payment specifics).
- Outbound webhooks you send: sign `${timestamp}.${rawBody}` with HMAC and ship timestamp and signature together in one header, constant-time compare on receipt, reveal the signing secret exactly once, and expose the delivery/retry log to the subscriber.
- Message payloads are versioned objects with a type discriminator, not bare values — a new field can't break existing consumers.
- A subscriber gets its own dedicated connection: in brokers where a subscribed connection can't issue normal commands (e.g. Redis pub/sub — duplicate the client), sharing one connection deadlocks the app. Guard the message parse and drop malformed messages instead of crashing the consume loop.
- Ordering isn't guaranteed across partitions/consumers. If order matters, key by entity and process one key serially, or carry a sequence number the consumer checks.
- Idempotent also means order-independent: a redelivered, delayed, or backfilled message must not clobber newer state — merge on the entity's version/timestamp carried in the payload, never on arrival order. Last-write-wins by arrival silently undoes the newest write during any replay.
