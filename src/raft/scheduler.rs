use std::time::Duration;
use rand::{rng, Rng};
use tokio::sync::mpsc::Sender;
use tokio::time;
use crate::raft::model::inner_messaging::NodeMessage;

#[derive(Debug)]
pub struct Scheduler {
    bus_tx: Sender<NodeMessage>
}

impl Scheduler {
    pub fn new(bus_tx: Sender<NodeMessage>) -> Scheduler {
        Scheduler {
            bus_tx
        }
    }

    pub async fn schedule_leader_election_process(&self, election_timeout_seconds: (u64, u64)) {
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

    // #[tracing::instrument(skip(self))]
    pub async fn schedule_heartbeats(&self, heartbeat_timeout_seconds: u64) {
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