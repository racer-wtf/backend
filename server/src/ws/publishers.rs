use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio::time;

use super::PubSubState;

/// Starts all publishers as threaded tasks
pub async fn run_publishers(state: Arc<PubSubState>) {
    let mut set = JoinSet::new();

    // publish online users
    set.spawn(publish_online(
        state.tx_online.clone(),
        state.online.clone(),
    ));

    // wait for all tasks to complete
    while let Some(res) = set.join_next().await {
        res.unwrap();
    }
}

/// Every 5 seconds, broadcasts the online user count
async fn publish_online(sender: broadcast::Sender<String>, online_count: Arc<AtomicU32>) {
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;
        let online_count = online_count.load(std::sync::atomic::Ordering::Relaxed);

        if sender.receiver_count() < 1 {
            continue;
        }

        tracing::trace!("publishing online count");

        if let Err(e) = sender.send(
            json!({
                "type": "online",
                "count": online_count,
            })
            .to_string(),
        ) {
            tracing::error!("failed to broadcast online count: {}", e);
        }
    }
}
