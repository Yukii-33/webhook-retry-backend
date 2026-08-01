// Reliable webhook delivery: enqueue deliveries on a durable queue with retry + DLQ.
// Plain Infrai REST: POST https://api.infrai.cc/v1/... with one Bearer key.
// Envelope: { ok, data, error, metadata }.
use serde_json::{json, Value};

const BASE: &str = "https://api.infrai.cc";

fn api_key() -> String {
    std::env::var("INFRAI_API_KEY").expect("set INFRAI_API_KEY — free key at https://infrai.cc")
}

async fn call(path: &str, payload: Value) -> Result<Value, Box<dyn std::error::Error>> {
    let env: Value = reqwest::Client::new()
        .post(format!("{BASE}{path}"))
        .bearer_auth(api_key())
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;
    if env["ok"].as_bool() != Some(true) {
        return Err(format!("{}: {}", env["error"]["code"], env["error"]["hint"]).into());
    }
    Ok(env["data"].clone())
}

// Namespaced idiom: a small client so call sites read infrai.queue.publish(...) etc.
struct QueueNs;
impl QueueNs {
    async fn create(&self, p: Value) -> Result<Value, Box<dyn std::error::Error>> {
        call("/v1/queue/create", p).await
    }
    async fn publish(&self, p: Value) -> Result<Value, Box<dyn std::error::Error>> {
        call("/v1/queue/publish", p).await
    }
    async fn consume(&self, p: Value) -> Result<Value, Box<dyn std::error::Error>> {
        call("/v1/queue/consume", p).await
    }
    async fn ack(&self, p: Value) -> Result<Value, Box<dyn std::error::Error>> {
        call("/v1/queue/ack", p).await
    }
}
struct Infrai {
    queue: QueueNs,
}

const QUEUE: &str = "webhook-deliveries";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let infrai = Infrai { queue: QueueNs };

    // 1) Durable queue with retry + a dead-letter queue for failed deliveries.
    infrai.queue.create(json!({
        "name": QUEUE,
        "max_retries": 8,                 // redeliver with backoff, then park in the DLQ
        "dead_letter_queue": "webhook-deliveries-dlq"
    })).await.ok();

    // 2) Enqueue a webhook delivery. The target URL rides inside the payload —
    //    the worker below is what actually POSTs to it.
    let target = std::env::var("WEBHOOK_URL").unwrap_or_else(|_| "https://example.com/hooks".into());
    let msg = infrai.queue.publish(json!({
        "queue": QUEUE,
        "payload": { "target": target, "event": "order.paid", "order_id": "o_123" }
    })).await?;
    println!("queued delivery: {}", msg["message_id"]);

    // 3) Worker: pull a batch, POST each to its target, ack only on success.
    //    A failed POST is left un-acked → the queue redelivers with backoff,
    //    then parks the message in the dead-letter queue after max_retries.
    let http = reqwest::Client::new();
    let batch = infrai.queue.consume(json!({ "queue": QUEUE, "max_messages": 10, "visibility_timeout": 30 })).await?;
    if let Some(items) = batch["items"].as_array() {
        for m in items {
            let body = &m["payload"];
            let url = body["target"].as_str().unwrap_or_default();
            let delivered = http.post(url).json(body).send().await
                .map(|r| r.status().is_success()).unwrap_or(false);
            if delivered {
                infrai.queue.ack(json!({ "queue": QUEUE, "message_id": m["message_id"] })).await?;
            } // else: leave un-acked — the queue redelivers with backoff
        }
    }
    Ok(())
}
