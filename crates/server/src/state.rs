use std::collections::HashSet;

use tokio::{
    select,
    sync::{mpsc, oneshot, watch},
    time::{sleep, Duration},
};

use tracing::{error, info};

pub struct State {
    ipfs_hashes: HashSet<String>,
}

#[derive(Clone)]
pub struct StateClient {
    req_tx: mpsc::Sender<StateRequest>,
    stop_tx: watch::Sender<()>,
}

#[derive(Debug)]
pub struct StateRequest {
    payload: StateRequestPayload,
    sender: oneshot::Sender<Result<StateResponse, StateClientError<StateRequest>>>,
}

#[derive(Debug)]
enum StateRequestPayload {
    AddIpfsHash { hash: String },
    GetIpfsHashes,
}

#[derive(Debug)]
enum StateResponse {
    AddIpfsHash,
    GetIpfsHashes { hashes: Vec<String> },
}

impl StateClient {
    pub fn new(req_tx: mpsc::Sender<StateRequest>, stop_tx: watch::Sender<()>) -> Self {
        Self { req_tx, stop_tx }
    }

    pub async fn stopped(self) {
        self.stop_tx.closed().await
    }

    pub fn stop(&self) -> Result<(), StateClientError<()>> {
        self.stop_tx.send(())?;
        Ok(())
    }

    pub async fn add_ipfs_hash(&self, hash: String) -> Result<(), StateClientError<StateRequest>> {
        let payload = StateRequestPayload::AddIpfsHash { hash };
        self.send_request(payload).await?;
        Ok(())
    }

    pub async fn get_ipfs_hashes(&self) -> Result<Vec<String>, StateClientError<StateRequest>> {
        let payload = StateRequestPayload::GetIpfsHashes;
        let StateResponse::GetIpfsHashes { hashes } = self.send_request(payload).await? else {
            return Err(StateClientError::UnexpectedResponse);
        };
        Ok(hashes)
    }

    async fn send_request(
        &self,
        payload: StateRequestPayload,
    ) -> Result<StateResponse, StateClientError<StateRequest>> {
        let (sender, receiver) =
            oneshot::channel::<Result<StateResponse, StateClientError<StateRequest>>>();
        let req = StateRequest { payload, sender };

        self.req_tx
            .send(req)
            .await
            .map_err(|err| StateClientError::MpscSend { source: err })?;

        let resp = Self::receive_response(receiver).await?;

        Ok(resp)
    }

    async fn receive_response(
        receiver: oneshot::Receiver<Result<StateResponse, StateClientError<StateRequest>>>,
    ) -> Result<StateResponse, StateClientError<StateRequest>> {
        select! {
            _ = sleep(Duration::from_secs(5)) => {
                Err(StateClientError::Timeout)
            },
            msg = receiver => {
                match msg {
                    Ok(resp) => Ok(resp),
                    Err(err) => Err(err.into())
                }
            }
        }?
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            ipfs_hashes: HashSet::new(),
        }
    }

    pub fn start(self) -> StateClient {
        let (req_tx, req_rx) = mpsc::channel::<StateRequest>(100);
        let (stop_tx, stop_rx) = watch::channel(());

        tokio::spawn(self.run(req_rx, stop_rx));
        StateClient::new(req_tx, stop_tx)
    }

    async fn run(self, req_rx: mpsc::Receiver<StateRequest>, mut stop_rx: watch::Receiver<()>) {
        select! {
            _ = self.listen(req_rx) => {
                error!("State stopped unexpectedly");
            },
            _ = stop_rx.changed() => {
                info!("State has shutdown after receiving message");
            }
        }
    }

    async fn listen(mut self, mut req_rx: mpsc::Receiver<StateRequest>) {
        while let Some(req) = req_rx.recv().await {
            let resp = match req.payload {
                StateRequestPayload::AddIpfsHash { hash } => {
                    self.ipfs_hashes.insert(hash);
                    Ok(StateResponse::AddIpfsHash)
                }
                StateRequestPayload::GetIpfsHashes => {
                    let hashes = self.ipfs_hashes.iter().cloned().collect::<Vec<String>>();
                    Ok(StateResponse::GetIpfsHashes { hashes })
                }
            };

            Self::send_response(resp, req.sender).await;
        }
    }

    async fn send_response(
        resp: Result<StateResponse, StateClientError<StateRequest>>,
        sender: oneshot::Sender<Result<StateResponse, StateClientError<StateRequest>>>,
    ) {
        if sender.send(resp).is_err() {
            error!("Error sending response to client. The receiver has been dropped");
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum StateClientError<T> {
    #[error("")]
    MpscSend {
        #[from]
        source: tokio::sync::mpsc::error::SendError<T>,
    },

    #[error("")]
    WatchSend {
        #[from]
        source: tokio::sync::watch::error::SendError<T>,
    },

    #[error("")]
    Recv {
        #[from]
        source: tokio::sync::oneshot::error::RecvError,
    },

    #[error("")]
    Timeout,

    #[error("")]
    UnexpectedResponse,
}
