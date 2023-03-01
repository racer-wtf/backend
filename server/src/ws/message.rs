use std::collections::HashSet;

use serde::Deserialize;
use serde_json::Result;

#[derive(PartialEq, Eq, Hash, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionType {
    Online,
    Leaderboard,
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
