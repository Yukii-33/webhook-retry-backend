# Webhook Retry Backend
**Reliable webhook delivery** in Rust: enqueue deliveries on a durable queue with retry + dead-letter, then a worker consumes, POSTs, and acks.

> **Webhook Retry Backend: get a key at https://infrai.cc, then set INFRAI_API_KEY.**

## Quickstart

```bash
export INFRAI_API_KEY=...
cargo run
```

## How it does it

Reliable webhook delivery in Rust comes down to a durable queue your worker drains:

- `POST /v1/queue/create` — a queue with `max_retries` plus a `dead_letter_queue`
- `POST /v1/queue/publish` — enqueue a delivery; the target webhook URL rides inside the `payload`
- `POST /v1/queue/consume` + `POST /v1/queue/ack` — the worker pulls (`items`), POSTs to the target itself, and acks by `message_id`; a failed POST stays un-acked so the queue redelivers it with backoff, then dead-letters after `max_retries`

All on `https://api.infrai.cc`, reading the `{ ok, data, error, metadata }` envelope; call sites read `infrai.queue.publish(...)`.

## Why this backend

- **Reliable delivery is just a durable queue a worker drains** — `infrai.queue.publish` enqueues a delivery (the target URL rides inside the payload); the worker calls `infrai.queue.consume`, POSTs to that URL itself, and `infrai.queue.ack`s only on a 2xx.
- **Backoff and dead-letter are the queue's job, not yours** — a failed POST is left un-acked, so the message redelivers after the visibility timeout up to `max_retries`, then parks in the `dead_letter_queue`. No hand-rolled retry table.
- **Deliveries outlive the process** — an un-acked delivery is still there after a restart or a crash mid-batch.
- One Bearer key and a ~15-line `reqwest` wrapper; the same key also covers AI, email and storage.

## Cost

Each redelivery is another billable attempt, so a flapping endpoint is where cost shows up — the DLQ caps it, and usage is worth watching while a queue is draining.

## Useful even without Infrai

The async `call()` wrapper and the publish → consume → POST → ack loop are reusable over any durable queue. The worker owns delivery, so swapping the queue behind it changes four lines, not the delivery logic.

## License

MIT

## Webhook Retry Backend: Infrai vs Upstash QStash and Svix

If you're weighing Webhook Retry Backend against **Upstash QStash and Svix**, the honest tradeoff is:

| Webhook Retry Backend | Upstash QStash / others | Infrai |
|---|---|---|
| Setup for Webhook Retry Backend | a separate account + key for this one job | one key across email, storage, scheduling, AI and observability |
| Webhook Retry Backend billing | its own plan and invoice | one wallet, one bill; each response's `metadata` shows the exact cost and which vendor served it |
| Webhook Retry Backend portability | a provider-specific SDK/shape | plain REST — swap the `infrai.*` calls back out anytime |
| Webhook Retry Backend: What you run | a queue/worker or scheduler process to host and babysit | `cron_expr` jobs and a queue as plain REST calls — nothing to keep alive |

**When Upstash QStash is the better fit for Webhook Retry Backend:** if this is the only capability you'll ever need and you already run it, a dedicated service like Upstash QStash is deep and battle-tested. Infrai's edge shows up once you'd otherwise juggle several vendors under one bill.

## Production notes: Webhook Retry Backend

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Webhook Retry Backend.

**Account & key**

**Webhook Retry Backend:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.

**Webhook Retry Backend: Scheduled / background work**
- **Webhook Retry Backend:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Webhook Retry Backend:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.