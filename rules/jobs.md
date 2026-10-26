# Scheduled jobs and batch work

Read this when writing cron jobs, scheduled tasks, or batch processing — anything that runs on a timer rather than on a request or a message.

- N replicas run N copies of an in-process schedule: single execution needs an external guarantee — the platform's singleton scheduler (e.g. one CronJob, one beat process) or a lock in shared storage, never a per-instance timer.
- A job lock is a lease with a TTL plus a fencing check, not a boolean: a crashed holder must not block the next run forever, and an expired holder must not land its write after a new holder started (`rules/resilience.md` fencing tokens).
- Overlap policy is explicit: a run that outlives its interval either skips the next tick, queues it, or kills the predecessor — pick one; running both concurrently corrupts shared state.
- Every job is an idempotent rerun: partial completion plus restart must converge — process in checkpointed batches keyed by stable ids and resume from the checkpoint, not from zero (`rules/messaging.md` idempotent consumers).
- Missed runs are a decision, not a scheduler default: after downtime either catch up (bounded, oldest first) or skip to now — the wrong implicit choice double-bills or double-mails.
- Schedule in UTC unless the task is inherently local-time; a local-timezone cron fires twice or zero times across DST transitions — when local time matters, use a DST-aware scheduler, not a fixed offset.
- Every job sets a hard deadline via the runner's timeout and reports terminal status — a hung job holding its lock is a silent outage of everything downstream of its schedule. A failing job also has a retry limit: one without it is a restart loop that floods logs and bills.
- Alert on absence, not only failure: a job that stopped being scheduled emits no error — track last-success age and alert when it exceeds the interval (`rules/observability.md`).
- Backfills are production writes at batch speed: throttle on replication lag and rate limits, reuse the live path's idempotency keys, and announce them like a deploy (`rules/database.md`).
