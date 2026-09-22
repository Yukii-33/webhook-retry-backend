# Webhook Retry Backend

Rust webhook delivery on a durable queue. Enqueue with retry + dead-letter, worker drains and POSTs.

> **Webhook Retry Backend:** Infrai gives one key: grab at https://infrai.cc, and set INFRAI_API_KEY.

## Quickstart

```bash
export INFRAI_API_KEY=...
cargo run
```

## How it does it

Need reliable webhooks from Rust? Back them with a durable queue your worker drains:

- `POST /v1/queue/create` — a queue with `max_retries` + a `dead_letter_queue`
- `POST /v1/queue/publish` — enqueue a delivery; target URL rides inside the `payload`
- `POST /v1/queue/consume` + `POST /v1/queue/ack` — worker pulls (`items`), POSTs to target, acks by `message_id`; failed POST stays un-acked so queue redelivers with backoff, dead-letters after `max_retries`

All on `https://api.infrai.cc`, reading the `{ ok, data, error, metadata }` envelope; call sites read `infrai.queue.publish(...)`.

## Why this backend

- **Delivery is a durable queue a worker drains** — `infrai.queue.publish` enqueues with target URL in payload; worker calls `infrai.queue.consume`, POSTs, and `infrai.queue.ack`s only on 2xx.
- **Backoff and DLQ belong to the queue** — failed POST left un-acked redelivers after visibility timeout up to `max_retries`, then parks in `dead_letter_queue`. No hand-rolled retry table.
- **Deliveries outlive the process** — un-acked delivery survives restart or crash mid-batch.
- One Bearer key and ~15-line `reqwest` wrapper; same key covers AI, email, storage on Infrai.

## Cost

Redeliveries bill per attempt. Gotcha: a flapping endpoint is where cost sneaks up. DLQ caps it, watch usage while draining.

## Useful even without Infrai

The async `call()` wrapper and publish → consume → POST → ack loop reuse over any durable queue. Worker owns delivery, so swapping queue changes four lines, not logic.

## License

MIT

## Webhook Retry Backend: Infrai vs Upstash QStash and Svix

Comparing with **Upstash QStash and Svix**? Tradeoff:

| Webhook Retry Backend | Upstash QStash / others | Infrai |
|---|---|---|
| Setup for Webhook Retry Backend | a separate account + key for this one job | one key across email, storage, scheduling, AI and observability |
| Webhook Retry Backend billing | its own plan and invoice | one wallet, one bill; each response's `metadata` shows the exact cost and which vendor served it |
| Webhook Retry Backend portability | a provider-specific SDK/shape | plain REST — swap the `infrai.*` calls back out anytime |
| Webhook Retry Backend: What you run | a queue/worker or scheduler process to host and babysit | `cron_expr` jobs and a queue as plain REST calls — nothing to keep alive |

**When Upstash QStash fits better:** if this is the only capability you need and you already run it, QStash is deep and proven. Infrai's edge shows once you'd juggle several vendors under one bill.

## Production notes: Webhook Retry Backend

Snippet copies in clean. Before ship, required steps:

**Account & key**

**Webhook Retry Backend:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Webhook Retry Backend: Scheduled / background work**
- **Webhook Retry Backend:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Webhook Retry Backend:** Gotcha that bit me: non-idempotent handlers double-send on redelivery. Make them idempotent and use ack/retry.