use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use titlecase::titlecase;
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

#[derive(Serialize, Deserialize)]
struct LeaderboardMessage {
    #[serde(rename = "type")]
    _type: String,
    leaderboard: Vec<Emoji>,
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

        let message = LeaderboardMessage {
            _type: "leaderboard".to_string(),
            leaderboard: generate_mock_leaderboard(),
        };
        match serde_json::to_string(&message) {
            Ok(new_leaderboard) => {
                let mut leaderboard = last_leaderboard.lock().await;
                *leaderboard = new_leaderboard;

                if let Err(e) = sender.send(leaderboard.clone()) {
                    tracing::error!("failed to broadcast leaderboard: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("failed to serialize leaderboard: {}", e);
                continue;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Emoji {
    emoji: String,
    label: String,
    value: u32,
}

fn generate_mock_leaderboard() -> Vec<Emoji> {
    let mut emojis = vec!["🌶️", "🔥", "🌞", "🦠", "🫐", "🍆", "🤍"];
    emojis.dedup(); // important!

    let mut leaderboard: Vec<Emoji> = emojis
        .iter()
        .map(|emoji| {
            let label = match emojis::get(emoji) {
                Some(emoji) => emoji.name().to_string(),
                None => "Unknown".to_string(),
            };
            Emoji {
                emoji: emoji.to_string(),
                label: titlecase(&label),
                value: rand::random::<u32>() % 100,
            }
        })
        .collect();

    // leaderboard.sort_by(|a, b| b.value.cmp(&a.value));

    leaderboard
}
