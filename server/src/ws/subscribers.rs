use tokio::sync::{broadcast::Receiver, mpsc::Sender};

/// Public subscription for broadcasted online count messages
pub async fn subscribe_online(mut receiver: Receiver<String>, sender: Sender<String>) {
    // subscribe to the online channel
    while let Ok(msg) = receiver.recv().await {
        // break if the message can't be sent
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}

/// Public subscription for broadcasted leaderboard messages
pub async fn subscribe_leaderboard(mut receiver: Receiver<String>, sender: Sender<String>) {
    while let Ok(msg) = receiver.recv().await {
        // break if the message can't be sent
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}

/// Private subscription for given address rewards
pub async fn subscribe_rewards(sender: Sender<String>, address: String) {
    sender.send(address).await.unwrap();
}

/// Private subscription for given address notification count
pub async fn subscribe_notifications(sender: Sender<String>, address: String) {
    sender.send(address).await.unwrap();
}
