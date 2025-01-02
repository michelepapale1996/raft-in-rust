use crate::raft::model::inner_messaging::NodeMessage;
use rand::{rng, Rng};
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time;

#[derive(Debug)]
pub struct AppendEntriesTimeoutEmitter {
    bus_tx: Sender<NodeMessage>,
}

impl AppendEntriesTimeoutEmitter {
    pub fn new(bus_tx: Sender<NodeMessage>) -> AppendEntriesTimeoutEmitter {
        AppendEntriesTimeoutEmitter { bus_tx }
    }

    // #[tracing::instrument(skip(self))]
    pub async fn start(&self, heartbeat_timeout_seconds: u64) {
        let sender = self.bus_tx.clone();

        tokio::spawn(async move {
            loop {
                time::sleep(Duration::from_millis(heartbeat_timeout_seconds * 1000)).await;

                let result = sender.send(NodeMessage::AppendEntriesTimeout).await;
                if let Err(x) = result {
                    tracing::error!("Error while sending the request");
                    tracing::error!("{:?}", x);
                }
            }
        });
    }
}
