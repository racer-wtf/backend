use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::sync::{broadcast, Mutex};
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

    set.spawn(publish_leaderboard(
        state.tx_leaderboard.clone(),
        state.leaderboard.clone(),
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

/// Every new mined block, broadcasts the leaderboard
async fn publish_leaderboard(
    sender: broadcast::Sender<String>,
    last_leaderboard: Arc<Mutex<String>>,
) {
    let mut interval = time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;
        if sender.receiver_count() < 1 {
            continue;
        }

        tracing::trace!("publishing leaderboard");

        let mut leaderboard = last_leaderboard.lock().await;
        *leaderboard = json!({
            "type": "online",
            "leaderboard": [
                {
                  "emoji": "🌶️",
                  "label": "Chili Pepper",
                  "value": 61,
                },
                {
                  "emoji": "🔥",
                  "label": "Fire",
                  "value": 40,
                },
                {
                  "emoji": "🌞",
                  "label": "Sun",
                  "value": 20,
                },
                {
                  "emoji": "🦠",
                  "label": "Microbe",
                  "value": 10,
                },
                {
                  "emoji": "🫐",
                  "label": "Blueberries",
                  "value": 4,
                },
                {
                  "emoji": "🍆",
                  "label": "Eggplant",
                  "value": 2,
                },
                {
                  "emoji": "🤍",
                  "label": "White Heart",
                  "value": 2,
                },
            ]
        })
        .to_string();

        if let Err(e) = sender.send(leaderboard.clone()) {
            tracing::error!("failed to broadcast online count: {}", e);
        }
    }
}
