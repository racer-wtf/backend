use std::ops::ControlFlow;
use std::{collections::HashSet, net::SocketAddr};

use axum::extract::ws::Message;
use serde::Deserialize;
use serde_json::Result;

pub fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(message) => {
            tracing::trace!(">>> {} sent str: {:?}", who, message);
        }
        Message::Binary(bytes) => {
            tracing::trace!(">>> {} sent {} bytes: {:?}", who, bytes.len(), bytes);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                tracing::trace!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who,
                    cf.code,
                    cf.reason
                );
            } else {
                tracing::trace!(">>> {} somehow sent close message without CloseFrame", who);
            }
            return ControlFlow::Break(());
        }
        Message::Pong(v) => {
            tracing::trace!(">>> {} sent pong with {:?}", who, v);
        }
        Message::Ping(v) => {
            tracing::trace!(">>> {} sent ping with {:?}", who, v);
        }
    }
    ControlFlow::Continue(())
}

#[derive(PartialEq, Eq, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionType {
    Leaderboard,
    Rewards,
    Notifications,
    Online,
}

#[derive(Debug, Deserialize)]
pub struct PubSubRequest {
    pub address: Option<String>,
    #[serde(default)]
    pub subscriptions: HashSet<SubscriptionType>,
}

pub fn process_subscription_message(message: impl ToString) -> Result<PubSubRequest> {
    serde_json::from_str(&message.to_string())
}
