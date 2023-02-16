use tokio::sync::{broadcast::Receiver, mpsc::Sender};

/// Subscription for broadcasted online count messages
pub async fn subscribe_online(mut receiver: Receiver<String>, sender: Sender<String>) {
    // subscribe to the online channel
    while let Ok(msg) = receiver.recv().await {
        // break if the message can't be sent
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}

/// Subscription for broadcasted leaderboard messages
pub async fn subscribe_leaderboard(mut receiver: Receiver<String>, sender: Sender<String>) {
    while let Ok(msg) = receiver.recv().await {
        // break if the message can't be sent
        if sender.send(msg).await.is_err() {
            break;
        }
    }
}
