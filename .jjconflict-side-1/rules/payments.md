# Payments and checkout

Read this when integrating any payment provider: checkout, webhooks, fulfillment, refunds, subscriptions.

- Never trust client-sent prices, currencies, or totals: the client sends product ids and quantities; the server re-derives every line item and amount from the canonical database/catalog before creating the payment session.
- Verify webhook signatures against the raw, unparsed request body with the provider SDK; reject (400) if the signature header is missing or invalid. A parsed then re-serialized body fails verification. The general inbound-webhook contract — replay window, fast 2xx, async work — is in `rules/messaging.md`; repeated here because the payment path is where getting it wrong costs money.
- Fulfill only from the provider's payment-succeeded webhook event, never on the browser success/redirect page — the redirect is display-only and unreliable (tab closed, network dropped). Mark the success page `noindex`.
- Fulfillment is idempotent: look up by the provider's session/event id and no-op if already processed — providers redeliver and retry events.
- Read idempotency and fulfillment state from the primary database, never a replica: replica lag means a redelivered event doesn't find the just-written record and fulfills twice — the classic double-charge mechanism (`rules/database.md` read-your-own-write rule).
- Webhook return codes drive provider retries: 400 on bad signature (no retry), 5xx on a handler exception (provider retries), 2xx only after the work is durably done. Never return 2xx on failure — that drops the event.
- Do all order writes (order, line items, stock decrement, cart clear) in one transaction; run side-effects (confirmation email, analytics) after commit, best-effort, catching their errors so they never roll back a paid order.
- Guard stock/inventory against oversell in the database (`GREATEST(stock - qty, 0)` or a conditional `UPDATE ... WHERE stock >= qty`), not read-modify-write in app code.
- Send the provider's idempotency key on create calls (charge, refund, session) so a retried request can't double-charge.
- Money is integer minor units end to end — never a float (rounding drift), never a locale-formatted string (parse ambiguity). Round once, at a documented point, and take the currency's exponent from the provider instead of assuming two decimals (JPY has none, KWD has three).
- Refunds, disputes, and chargebacks are first-class flows, not error handling: each gets its own webhook, its own idempotent handler, and an explicit decision about what it does to fulfillment — revoke access, restock, or neither.
- Subscriptions add a lifecycle the one-off flow lacks: trial ending, renewal, proration on plan change, payment failure and dunning, cancel-at-period-end versus cancel-now. Derive entitlement from the provider's current subscription state, never from the last event this service happened to receive.
- Reconcile against the provider's own reports on a schedule — paid-but-unfulfilled and fulfilled-but-unpaid are both silent failures until the two ledgers are compared.
- Rate-limit checkout initiation per client so an attacker can't run up provider API cost or spam sessions.
- Never store raw card data (PCI-DSS scope); keep the provider's secret API key and webhook signing secret server-only and out of the client bundle and any tracked file (`rules/backend-security.md`).
