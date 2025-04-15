use std::collections::HashSet;

use tokio::{
    select,
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    task::JoinHandle,
    time::{sleep, Duration},
};

use tracing::error;

pub struct ServerState {
    ipfs_hashes: HashSet<String>,
}

#[derive(Clone)]
pub struct StateClient {
    tx: Sender<StateRequest>,
}

pub struct StateRequest {
    payload: StateRequestPayload,
    sender: oneshot::Sender<Result<StateResponse, String>>,
}
enum StateRequestPayload {
    AddIpfsHash { hash: String },
}

enum StateResponse {
    AddIpfsHash,
}

impl StateClient {
    pub fn new(tx: Sender<StateRequest>) -> Self {
        Self { tx }
    }

    pub async fn add_ipfs_hash(&self, hash: String) -> Result<(), ()> {
        let payload = StateRequestPayload::AddIpfsHash { hash };
        self.send_request(payload).await;
        Ok(())
    }

    async fn send_request(&self, payload: StateRequestPayload) {
        let (sender, receiver) = oneshot::channel::<Result<StateResponse, String>>();
        let req = StateRequest { payload, sender };

        self.tx.send(req).await.unwrap();

        let foo = Self::receive_response(receiver).await;
    }

    async fn receive_response(
        receiver: oneshot::Receiver<Result<StateResponse, String>>,
    ) -> Result<StateResponse, String> {
        select! {
            _ = sleep(Duration::from_secs(5)) => {
                Err(String::from("foo"))
            },
            msg = receiver => {
                match msg {
                    Ok(resp) => Ok(resp),
                    Err(err) => Err(String::from("foo"))
                }
            }
        }
        .unwrap()
    }
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            ipfs_hashes: HashSet::new(),
        }
    }

    pub fn start(self) -> (JoinHandle<()>, StateClient) {
        let (tx, rx) = channel::<StateRequest>(100);
        let state_handle = tokio::spawn(self.listen(rx));
        let state_client = StateClient::new(tx);

        (state_handle, state_client)
    }

    async fn listen(mut self, mut rx: Receiver<StateRequest>) {
        while let Some(req) = rx.recv().await {
            let resp = match req.payload {
                StateRequestPayload::AddIpfsHash { hash } => {
                    self.ipfs_hashes.insert(hash);
                    Ok(StateResponse::AddIpfsHash)
                }
            };

            Self::send_response(resp, req.sender).await;
        }
    }

    async fn send_response(
        resp: Result<StateResponse, String>,
        sender: oneshot::Sender<Result<StateResponse, String>>,
    ) {
        if sender.send(resp).is_err() {
            error!("Error sending response to client. The receiver has been dropped");
        }
    }
}
