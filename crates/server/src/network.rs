use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io,
    str::FromStr,
    time::Duration,
};

use futures::StreamExt;
use libp2p::{
    gossipsub,
    multiaddr::Protocol,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};

use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot, watch},
    task::yield_now,
    time::Duration as TokioDuration,
};
use tracing::{error, info, warn};

type GossipMessage = Vec<u8>;
pub(crate) struct NoP;
pub(crate) struct NoT;
pub(crate) struct NoA;

pub struct NetworkBuilder<P, T, A> {
    port: P,
    is_boot_node: T,
    boot_addr: A,
}

pub struct Network {
    swarm: Swarm<Behavior>,
    port: String,
    is_boot_node: bool,
    boot_addr: String,
}

#[derive(Clone)]
pub struct NetworkClient {
    req_tx: mpsc::Sender<ClientRequest>,
    gossip_msg_tx: broadcast::Sender<GossipMessage>,
    stop_tx: watch::Sender<()>,
}

impl NetworkBuilder<NoP, NoT, NoA> {
    pub fn new() -> Self {
        Self {
            port: NoP,
            is_boot_node: NoT,
            boot_addr: NoA,
        }
    }
}

impl<P, T, A> NetworkBuilder<P, T, A> {
    pub fn with_port(self, port: impl Into<String>) -> NetworkBuilder<String, T, A> {
        NetworkBuilder {
            port: port.into(),
            is_boot_node: self.is_boot_node,
            boot_addr: self.boot_addr,
        }
    }

    pub fn with_is_boot_node(self, is_boot_node: bool) -> NetworkBuilder<P, bool, A> {
        NetworkBuilder {
            port: self.port,
            is_boot_node,
            boot_addr: self.boot_addr,
        }
    }

    pub fn with_boot_addr(self, boot_addr: impl Into<String>) -> NetworkBuilder<P, T, String> {
        NetworkBuilder {
            port: self.port,
            is_boot_node: self.is_boot_node,
            boot_addr: boot_addr.into(),
        }
    }
}

impl NetworkBuilder<String, bool, String> {
    pub fn build(self) -> Result<Network, NetworkError> {
        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::tls::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s);
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(10))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .message_id_fn(message_id_fn)
                    .build()
                    .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?;

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )?;

                Ok(Behavior { gossipsub })
            })
            .map_err(|err| NetworkError::Behavior(err.to_string()))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build();

        Ok(Network {
            swarm,
            port: self.port,
            is_boot_node: self.is_boot_node,
            boot_addr: self.boot_addr,
        })
    }
}

impl NetworkClient {
    fn new(
        req_tx: mpsc::Sender<ClientRequest>,
        gossip_msg_tx: broadcast::Sender<GossipMessage>,
        stop_tx: watch::Sender<()>,
    ) -> Self {
        Self {
            req_tx,
            gossip_msg_tx,
            stop_tx,
        }
    }

    pub async fn stopped(self) {
        self.stop_tx.closed().await
    }

    pub async fn gossip_receiver(&self) -> broadcast::Receiver<GossipMessage> {
        self.gossip_msg_tx.subscribe()
    }

    pub async fn subscribe(&self, topic: String) -> Result<(), NetworkError> {
        let payload = ClientRequestPayload::Subscribe { topic };
        let ClientResponse::Subscribe = self.send_request(payload).await? else {
            return Err(NetworkError::UnexpectedResponse);
        };

        Ok(())
    }

    async fn send_request(
        &self,
        payload: ClientRequestPayload,
    ) -> Result<ClientResponse, NetworkError> {
        let (sender, receiver) = oneshot::channel::<Result<ClientResponse, NetworkError>>();
        let req = ClientRequest { payload, sender };

        self.req_tx
            .send(req)
            .await
            .map_err(|err| NetworkError::MpscSend(err.to_string()))?;

        let resp = Self::receive_response(receiver).await?;

        Ok(resp)
    }

    async fn receive_response(
        receiver: oneshot::Receiver<Result<ClientResponse, NetworkError>>,
    ) -> Result<ClientResponse, NetworkError> {
        select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                Err(NetworkError::Timeout)
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

impl Network {
    pub async fn start(mut self) -> Result<NetworkClient, NetworkError> {
        let (req_tx, req_rx) = mpsc::channel::<ClientRequest>(100);
        let (gossip_msg_tx, _) = broadcast::channel::<GossipMessage>(100);
        let (stop_tx, stop_rx) = watch::channel(());

        let addr = format!("/ip4/0.0.0.0/tcp/{}", self.port);

        self.swarm.listen_on(addr.parse()?)?;
        self.get_listener_addresses().await?;

        if !self.is_boot_node {
            self.dial_bootnode();
        }

        let network_client = NetworkClient::new(req_tx, gossip_msg_tx.clone(), stop_tx);

        tokio::spawn(async move { self.run(req_rx, gossip_msg_tx, stop_rx).await });

        Ok(network_client)
    }

    async fn get_listener_addresses(&mut self) -> Result<(), NetworkError> {
        let peer_id = PeerId::from_bytes(&self.swarm.local_peer_id().to_bytes())?;

        loop {
            select! {
                event = self.swarm.select_next_some() => {
                    if let SwarmEvent::NewListenAddr { address, .. } = event {
                        let full = address.with(Protocol::P2p(peer_id));
                        info!("Local node is listening on {full}");
                    }
                },
                _ = tokio::time::sleep(TokioDuration::from_millis(50)) => {
                    break
                }
            }
        }

        Ok(())
    }

    fn dial_bootnode(&mut self) {
        match Multiaddr::from_str(&self.boot_addr) {
            Ok(addr) => {
                if let Err(err) = self.swarm.dial(addr) {
                    warn!("Unable to dial boot addr: {}", err)
                }
            }
            Err(err) => {
                warn!("Unable to parse boot addr into Multiaddr: {}", err)
            }
        }
    }

    async fn run(
        &mut self,
        mut req_rx: mpsc::Receiver<ClientRequest>,
        gossip_msg_tx: broadcast::Sender<GossipMessage>,
        mut stop_rx: watch::Receiver<()>,
    ) -> Result<(), NetworkError> {
        loop {
            select! {
                Some(request) = req_rx.recv() => self.handle_client_request(request),
                event = self.swarm.select_next_some() => self.handle_event(event, &gossip_msg_tx).await,
                _ = stop_rx.changed() => break Ok(())
            }
        }
    }

    fn handle_client_request(&mut self, request: ClientRequest) {
        let sender = request.sender;
        match request.payload {
            ClientRequestPayload::Publish { topic, msg } => {
                let tpc = gossipsub::IdentTopic::new(&topic);
                let result =
                    if let Err(err) = self.swarm.behaviour_mut().gossipsub.publish(tpc, msg) {
                        error!("Publishing Error: {}", err);
                        Err(NetworkError::from(err))
                    } else {
                        info!("Successfully published message to {} topic", topic);
                        Ok(ClientResponse::Publish)
                    };
                Self::send_client_response(result, sender);
            }
            ClientRequestPayload::Subscribe { topic } => {
                let topic = gossipsub::IdentTopic::new(topic);

                let result =
                    if let Err(err) = self.swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                        error!("Subscription Error: {}", err);
                        Err(NetworkError::from(err))
                    } else {
                        info!("Subscribed to topic: {}", topic);
                        Ok(ClientResponse::Subscribe)
                    };
                Self::send_client_response(result, sender);
            }
        };
    }

    fn send_client_response(
        result: Result<ClientResponse, NetworkError>,
        sender: oneshot::Sender<Result<ClientResponse, NetworkError>>,
    ) {
        if sender.send(result).is_err() {
            error!("Error sending response to client. The receiver has been dropped");
        }
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<BehaviorEvent>,
        gossip_msg_tx: &broadcast::Sender<GossipMessage>,
    ) {
        match event {
            SwarmEvent::Behaviour(BehaviorEvent::Gossipsub(gossipsub::Event::Message {
                message,
                ..
            })) => match gossip_msg_tx.send(message.data) {
                Ok(_) => {
                    info!("Gossip message relayed to client");
                }
                Err(err) => error!("Error relaying gossip message to client: {}", err),
            },

            SwarmEvent::Behaviour(BehaviorEvent::Gossipsub(gossipsub::Event::Subscribed {
                peer_id: _peer_id,
                topic,
            })) => info!("A remote subscribed to a topic: {topic}"),
            _ => {}
        }
        yield_now().await;
    }
}

#[derive(NetworkBehaviour)]
struct Behavior {
    gossipsub: gossipsub::Behaviour,
}

struct ClientRequest {
    payload: ClientRequestPayload,
    sender: oneshot::Sender<Result<ClientResponse, NetworkError>>,
}
pub enum ClientRequestPayload {
    Publish { topic: String, msg: Vec<u8> },
    Subscribe { topic: String },
}

pub enum ClientResponse {
    Publish,
    Subscribe,
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("{source}")]
    Rcgen {
        #[from]
        source: libp2p::tls::certificate::GenError,
    },

    #[error("{source}")]
    MultiAddr {
        #[from]
        source: libp2p::multiaddr::Error,
    },

    #[error("{source}")]
    Transport {
        #[from]
        source: libp2p::TransportError<std::io::Error>,
    },

    #[error("{source}")]
    Subscription {
        #[from]
        source: libp2p::gossipsub::SubscriptionError,
    },

    #[error("{source}")]
    Publish {
        #[from]
        source: libp2p::gossipsub::PublishError,
    },

    #[error("{source}")]
    Parse {
        #[from]
        source: libp2p::identity::ParseError,
    },

    #[error("{source}")]
    Recv {
        #[from]
        source: tokio::sync::oneshot::error::RecvError,
    },

    #[error("Timeout")]
    Timeout,

    #[error("UnexpectedResponse")]
    UnexpectedResponse,

    #[error("MpscSend Error: {0}")]
    MpscSend(String),

    #[error("Behavior Error: {0}")]
    Behavior(String),
}
