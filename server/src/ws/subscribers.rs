use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::{broadcast::Receiver, mpsc::Sender, Mutex};

/// Subscription for broadcasted online count messages
pub async fn subscribe_online(
    mut receiver: Receiver<String>,
    sender: Sender<String>,
    online_count: Arc<AtomicU32>,
) {
    // get initial data
    let online_count = online_count.load(std::sync::atomic::Ordering::Relaxed);
    if sender
        .send(
            json!({
                "type": "online",
                "count": online_count,
            })
            .to_string(),
        )
        .await
        .is_err()
    {
        return;
    }

    // subscribe to the online channel
    while let Ok(msg) = receiver.recv().await {
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}

/// Subscription for broadcasted leaderboard messages
pub async fn subscribe_leaderboard(
    mut receiver: Receiver<String>,
    sender: Sender<String>,
    leaderboard: Arc<Mutex<String>>,
) {
    // send initial data
    if sender.send(leaderboard.lock().await.clone()).await.is_err() {
        return;
    }

    // subscribe to the leaderboard channel
    while let Ok(msg) = receiver.recv().await {
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}
