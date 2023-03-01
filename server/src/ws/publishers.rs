use std::error::Error;
use tokio::time::timeout;
use std::panic;
use std::str;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;

use bigdecimal::BigDecimal;
use bigdecimal::ToPrimitive;
use database::Database;
use ethers::providers::{Http, Middleware, Provider};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinSet;
use bytes::bytes_to_bigdecimal;
use tokio::time;

use super::PubSubState;

/// Starts all publishers as threaded tasks
pub async fn run_publishers(state: Arc<PubSubState>, database_url: &str, rpc_url: &str) {
    let mut set = JoinSet::new();

    // publish online users
    set.spawn(publish_online(
        state.tx_online.clone(),
        state.online.clone(),
    ));

    let leaderboard = Leaderboard::new(state.tx_leaderboard.clone(), database_url, rpc_url)
        .await
        .unwrap();
    set.spawn(async move {
        leaderboard.start(state.leaderboard.clone()).await;
    });

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
    cycle_id: i64,
    metadata: Metadata,
    leaderboard: Vec<Emoji>,
}

struct Leaderboard {
    sender: broadcast::Sender<String>,
    database: Database,
    eth_client: ethers::providers::Provider<Http>,
    chain_id: BigDecimal,
}

#[derive(Serialize, Deserialize)]
struct Emoji {
    emoji: String,
    value: u32,
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    blocks_remaining: u32,
    votes: i64,
    payout: String,
}

impl Leaderboard {
    pub async fn new(
        sender: broadcast::Sender<String>,
        database_url: &str,
        rpc_url: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let database = Database::new(database_url).await?;
        let eth_client = Provider::<Http>::try_from(rpc_url).expect("Could not connect to RPC");
        let chain_id = bytes_to_bigdecimal(eth_client.get_chainid().await?);

        Ok(Self {
            sender,
            database,
            eth_client,
            chain_id,
        })
    }

    /// Every new mined block, broadcasts the leaderboard
    async fn start(&self, last_leaderboard: Arc<Mutex<String>>) {
        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;
            if self.sender.receiver_count() < 1 {
                continue;
            }

            tracing::trace!("publishing leaderboard");
            if timeout(
                Duration::from_secs(5),
                self.publish_leaderboard(last_leaderboard.clone())
            ).await.is_err() {
                tracing::error!("failed to publish leaderboard");
            }
        }
    }

    async fn publish_leaderboard(&self, last_leaderboard: Arc<Mutex<String>>) {
        let cycle_id = match self.database.get_current_cycle(self.chain_id.clone()).await {
            Ok(cycle_id) => cycle_id,
            Err(error) => {
                tracing::error!("error fetching current cycle from database {:?}", error);
                return;
            }
        }.id.to_i64().unwrap();

        let metadata = match self.generate_metadata().await {
            Ok(metadata) => metadata,
            Err(error) => {
                tracing::error!(error);
                return;
            }
        };

        let leaderboard = match self.generate_leaderboard().await {
            Ok(leaderboard) => leaderboard,
            Err(error) => {
                tracing::error!(error);
                return;
            }
        };

        let message = LeaderboardMessage {
            _type: "leaderboard".to_string(),
            cycle_id,
            metadata,
            leaderboard,
        };

        match serde_json::to_string(&message) {
            Ok(new_leaderboard) => {
                let mut leaderboard = last_leaderboard.lock().await;
                *leaderboard = new_leaderboard;

                if let Err(e) = self.sender.send(leaderboard.clone()) {
                    tracing::error!("failed to broadcast leaderboard: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("failed to serialize leaderboard: {}", e);
            }
        }
    }

    async fn generate_metadata(&self) -> Result<Metadata, &str> {
        let Ok(cycle) = self.database.get_current_cycle(self.chain_id.clone()).await else { 
            return Err("Could not fetch current cycle")
        };
        let Ok(current_block) = self.eth_client.get_block_number().await else {
            return Err("Could not get current block from ethereum client")
        };
        let blocks_remaining = panic::catch_unwind(|| {
            (cycle.starting_block + cycle.block_length)
                .to_u32()
                .unwrap_or(0)
                - current_block.as_u32()
        })
        .unwrap_or(0);
        let Ok(votes) = self.database.get_vote_count(cycle.id, self.chain_id.clone()).await else {
            return Err("Could not fetch vote count from database")
        };
        let payout = (votes * cycle.vote_price.to_i64().unwrap_or(0)).to_string();

        Ok(Metadata {
            blocks_remaining,
            votes,
            payout,
        })
    }

    async fn generate_leaderboard(&self) -> Result<Vec<Emoji>, &str> {
        let Ok(cycle) = self.database.get_current_cycle(self.chain_id.clone()).await else {
            return Err("Could not get current cycle from the database")
        };
        let Ok(leaderboard) = self.database.get_leaderboard(cycle.id, self.chain_id.clone()).await else {
            return Err("Could not get leaderboard from the database")
        };

        let leaderboard: Vec<Emoji> = leaderboard
            .iter()
            .map(|emoji| Emoji {
                emoji: str::from_utf8(&emoji.symbol)
                    .unwrap_or("?")
                    .to_owned()
                    .chars()
                    .next()
                    .unwrap_or('?')
                    .to_string(),
                value: emoji
                    .amount
                    .clone()
                    .unwrap_or(BigDecimal::default())
                    .to_u32()
                    .unwrap_or(0),
            })
            .collect();

        Ok(leaderboard)
    }
}

#[allow(dead_code)]
fn generate_mock_leaderboard() -> Vec<Emoji> {
    let mut emojis = vec!["🌶️", "🔥", "🌞", "🦠", "🫐", "🍆", "🌸", "🤍"];
    emojis.dedup(); // important!

    let leaderboard: Vec<Emoji> = emojis
        .iter()
        .map(|emoji| Emoji {
            emoji: emoji.to_string(),
            value: rand::random::<u32>() % 1000,
        })
        .collect();

    leaderboard
}


#[allow(dead_code)]
fn generate_mock_metadata() -> Metadata {
    Metadata {
        blocks_remaining: rand::random::<u32>() % 100,
        votes: rand::random::<i64>() % 100,
        payout: (rand::random::<i64>() % 100000000000).to_string(),
    }
}
