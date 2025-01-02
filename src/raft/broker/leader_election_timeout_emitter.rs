use crate::raft::model::inner_messaging::NodeMessage;
use rand::Rng;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time;

#[derive(Debug)]
pub struct LeaderElectionTimeoutEmitter {
    bus_tx: Sender<NodeMessage>,
}

impl LeaderElectionTimeoutEmitter {
    pub fn new(bus_tx: Sender<NodeMessage>) -> Self {
        Self { bus_tx }
    }

    pub async fn start(&self, election_timeout_seconds: (u64, u64)) {
        let sender = self.bus_tx.clone();

        let sleep_time_millis = {
            let mut rng = rand::rng();
            let lower_bound = election_timeout_seconds.0 * 1000;
            let upper_bound = election_timeout_seconds.1 * 1000;
            let sleep_time_millis = rng.random_range(lower_bound..upper_bound);
            sleep_time_millis
        };

        tokio::spawn(async move {
            loop {
                time::sleep(Duration::from_millis(sleep_time_millis)).await;

                let result = sender.send(NodeMessage::ElectionTimeout).await;
                if let Err(x) = result {
                    tracing::error!("Error while sending the request");
                    tracing::error!("{:?}", x);
                }
            }
        });
    }
}
