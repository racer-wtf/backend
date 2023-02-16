use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use axum::extract::ws::Message;
use axum::extract::{ws::WebSocket, State};
use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinSet;

use crate::ws::message::SubscriptionType;

use super::message::PubSubRequest;
use super::{process_subscription_message, subscribe_leaderboard, subscribe_online};

pub struct PubSubState {
    // the count of users connected to ws server
    pub online: Arc<AtomicU32>,
    // channel that sends information about online users
    pub tx_online: broadcast::Sender<String>,
    // channel that sends information about leaderboard
    pub tx_leaderboard: broadcast::Sender<String>,
}

impl Default for PubSubState {
    fn default() -> Self {
        Self {
            online: Arc::new(AtomicU32::new(0)),
            tx_online: broadcast::channel(10_000).0,
            tx_leaderboard: broadcast::channel(10_000).0,
        }
    }
}

/// Upgrades an HTTP(s) connection to a websocket connection
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<PubSubState>>,
    ConnectInfo(socket_address): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket(socket, state, socket_address))
}

/// Handles websocket connections by processing incoming messages as subscription requests and
/// sending outgoing messages to subscribed users
async fn websocket(stream: WebSocket, state: Arc<PubSubState>, socket_address: SocketAddr) {
    // split the stream to allow for simultaneous sending and receiving
    let (mut sink, mut stream) = stream.split();

    // create an mpsc so we can send messages to the stream from multiple threads
    let (sender, mut receiver) = mpsc::channel::<String>(1000);

    // add 1 to online count
    state.online.fetch_add(1, Ordering::Relaxed);

    let mut socket_join_set = JoinSet::new();

    // since SplitSinks are not thread safe, we create an mpsc channel that will forward
    // messages to the SplitSink (`sink`)
    socket_join_set.spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(e) = sink.send(Message::Text(message)).await {
                tracing::trace!("error sending message to {}: {}", socket_address, e);
                break;
            }
        }
    });

    // listens for new messages from the user to update their subscription
    let recv_task_sender = sender.clone();
    let state_clone = state.clone();
    socket_join_set.spawn(async move {
        let mut subscription_join_set = JoinSet::new();

        while let Some(Ok(Message::Text(text))) = stream.next().await {
            match process_subscription_message(text) {
                Ok(result) => {
                    tracing::trace!(
                        "{}: eth address {:?} subscribed to {:?}",
                        socket_address,
                        result.address,
                        result.subscriptions
                    );

                    // remove old subscriptions
                    subscription_join_set.abort_all();

                    // create new subscriptions
                    subscription_join_set =
                        create_subscriptions(result, state_clone.clone(), recv_task_sender.clone());
                }
                Err(e) => {
                    tracing::info!(
                        "error processing subscription message from {}: {}",
                        socket_address,
                        e
                    );
                    if recv_task_sender.send(e.to_string()).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // blocks until any task finishes
    socket_join_set.join_next().await;
    socket_join_set.abort_all();
    tracing::trace!("websocket closed for {}", socket_address);

    // clean up when connection closes
    state.online.fetch_sub(1, Ordering::Relaxed);
}

fn create_subscriptions(
    result: PubSubRequest,
    state: Arc<PubSubState>,
    sender: mpsc::Sender<String>,
) -> JoinSet<()> {
    let mut join_set = JoinSet::new();

    result
        .subscriptions
        .iter()
        .for_each(|subscription| match subscription {
            SubscriptionType::Online => {
                let receiver = state.tx_online.subscribe();
                let sender = sender.clone();
                join_set.spawn(subscribe_online(receiver, sender));
            }
            SubscriptionType::Leaderboard => {
                let receiver = state.tx_leaderboard.subscribe();
                let sender = sender.clone();
                join_set.spawn(subscribe_leaderboard(receiver, sender));
            }
        });

    join_set
}
