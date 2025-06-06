use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io,
    str::FromStr,
    time::Duration,
};

use futures::{future::BoxFuture, StreamExt};
use libp2p::{
    gossipsub, identify, kad,
    multiaddr::Protocol,
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, Swarm,
};

use tokio::{
    select,
    sync::{broadcast, mpsc, oneshot, watch},
    task::yield_now,
    time::{timeout, Duration as TokioDuration},
};
use tracing::{error, info, warn, Instrument, Span};

type GossipMessage = Vec<u8>;
pub type GossipCallBackFn = Box<dyn for<'a> Fn(&'a [u8]) -> BoxFuture<'a, ()> + Send + Sync>;
pub struct NoP;
pub struct NoB;
pub struct NoA;
pub struct NoT;

pub struct NetworkBuilder<P, B, A, T> {
    port: P,
    is_boot_node: B,
    boot_addr: A,
    topic: T,
}

pub struct Network {
    swarm: Swarm<Behavior>,
    port: String,
    is_boot_node: bool,
    boot_addr: String,
    topic: String,
}

#[derive(Clone)]
pub struct NetworkClient {
    req_tx: mpsc::Sender<ClientRequest>,
    gossip_msg_tx: broadcast::Sender<GossipMessage>,
    stop_tx: watch::Sender<()>,
    topic: String,
}

impl NetworkBuilder<NoP, NoB, NoA, NoT> {
    pub fn new() -> Self {
        Self {
            port: NoP,
            is_boot_node: NoB,
            boot_addr: NoA,
            topic: NoT,
        }
    }
}

impl Default for NetworkBuilder<NoP, NoB, NoA, NoT> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P, B, A, T> NetworkBuilder<P, B, A, T> {
    pub fn with_port(self, port: impl Into<String>) -> NetworkBuilder<String, B, A, T> {
        NetworkBuilder {
            port: port.into(),
            is_boot_node: self.is_boot_node,
            boot_addr: self.boot_addr,
            topic: self.topic,
        }
    }

    pub fn with_is_boot_node(self, is_boot_node: bool) -> NetworkBuilder<P, bool, A, T> {
        NetworkBuilder {
            port: self.port,
            is_boot_node,
            boot_addr: self.boot_addr,
            topic: self.topic,
        }
    }

    pub fn with_boot_addr(self, boot_addr: impl Into<String>) -> NetworkBuilder<P, B, String, T> {
        NetworkBuilder {
            port: self.port,
            is_boot_node: self.is_boot_node,
            boot_addr: boot_addr.into(),
            topic: self.topic,
        }
    }

    pub fn with_topic(self, topic: impl Into<String>) -> NetworkBuilder<P, B, A, String> {
        NetworkBuilder {
            port: self.port,
            is_boot_node: self.is_boot_node,
            boot_addr: self.boot_addr,
            topic: topic.into(),
        }
    }
}

impl NetworkBuilder<String, bool, String, String> {
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

                let public_key = key.public();

                let local_id = public_key.to_peer_id();
                let store = kad::store::MemoryStore::new(local_id);
                let mut kademlia = kad::Behaviour::new(local_id, store);
                kademlia.set_mode(Some(kad::Mode::Server));

                let identify_config =
                    identify::Config::new("/local_ipfs/id/0.0.0".into(), public_key);
                let identify = identify::Behaviour::new(identify_config);

                Ok(Behavior {
                    gossipsub,
                    kademlia,
                    identify,
                })
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
            topic: self.topic,
        })
    }
}

impl NetworkClient {
    fn new(
        req_tx: mpsc::Sender<ClientRequest>,
        gossip_msg_tx: broadcast::Sender<GossipMessage>,
        stop_tx: watch::Sender<()>,
        topic: impl Into<String>,
    ) -> Self {
        Self {
            req_tx,
            gossip_msg_tx,
            stop_tx,
            topic: topic.into(),
        }
    }

    pub async fn stopped(&self) {
        self.stop_tx.closed().await
    }

    pub fn stop(&self) -> Result<(), NetworkError> {
        self.stop_tx
            .send(())
            .map_err(|err| NetworkError::WatchSend { source: err })?;
        Ok(())
    }

    pub async fn get_peer_id(&self) -> Result<PeerId, NetworkError> {
        let payload = ClientRequestPayload::PeerId;
        let ClientResponse::PeerId { peer_id } = self.send_request(payload).await? else {
            return Err(NetworkError::UnexpectedResponse);
        };

        Ok(peer_id)
    }

    pub async fn get_connected_peers(&self) -> Result<Vec<PeerId>, NetworkError> {
        let payload = ClientRequestPayload::ConnectedPeers;
        let ClientResponse::ConnectedPeers { peers } = self.send_request(payload).await? else {
            return Err(NetworkError::UnexpectedResponse);
        };

        Ok(peers)
    }

    pub async fn gossip_receiver(&self) -> broadcast::Receiver<GossipMessage> {
        self.gossip_msg_tx.subscribe()
    }

    pub async fn subscribe(&self) -> Result<(), NetworkError> {
        let payload = ClientRequestPayload::Subscribe {
            topic: self.topic.clone(),
        };
        let ClientResponse::Subscribe = self.send_request(payload).await? else {
            return Err(NetworkError::UnexpectedResponse);
        };

        Ok(())
    }

    pub async fn publish(&self, msg: Vec<u8>) -> Result<(), NetworkError> {
        let payload = ClientRequestPayload::Publish {
            topic: self.topic.clone(),
            msg,
        };

        let ClientResponse::Publish = self.send_request(payload).await? else {
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
        let duration = TokioDuration::from_secs(5);
        timeout(duration, async {
            match receiver.await {
                Ok(resp) => resp,
                Err(err) => Err(err.into()),
            }
        })
        .await
        .map_err(|_| NetworkError::Timeout)?
    }
}

impl Network {
    pub async fn start(
        mut self,
        gossip_callback_fns: Vec<GossipCallBackFn>,
    ) -> Result<NetworkClient, NetworkError> {
        let (req_tx, req_rx) = mpsc::channel::<ClientRequest>(100);
        let (gossip_msg_tx, gossip_msg_rx) = broadcast::channel::<GossipMessage>(100);
        let (stop_tx, stop_rx) = watch::channel(());

        let network_client =
            NetworkClient::new(req_tx, gossip_msg_tx.clone(), stop_tx, &self.topic);

        self.swarm
            .listen_on(format!("/ip4/0.0.0.0/tcp/{}", self.port).parse()?)?;
        self.wait_listener_addresses().await?;

        if !self.is_boot_node {
            self.dial_bootnode().await;
        }

        let span = Span::current();
        tokio::spawn(
            async move { Self::start_gossip_hanlder(gossip_msg_rx, gossip_callback_fns).await }
                .instrument(span.clone()),
        );

        tokio::spawn(
            async move { Self::run(self.swarm, req_rx, gossip_msg_tx, stop_rx).await }
                .instrument(span),
        );

        network_client.subscribe().await?;

        Ok(network_client)
    }

    async fn start_gossip_hanlder(
        mut gossip_msg_rx: broadcast::Receiver<Vec<u8>>,
        gossip_callback_fns: Vec<GossipCallBackFn>,
    ) {
        while let Ok(msg) = gossip_msg_rx.recv().await {
            for func in &gossip_callback_fns {
                func(&msg).await;
            }
        }
    }

    async fn wait_listener_addresses(&mut self) -> Result<(), NetworkError> {
        let peer_id = PeerId::from_bytes(&self.swarm.local_peer_id().to_bytes())?;

        loop {
            select! {
                event = self.swarm.select_next_some() => {
                    if let SwarmEvent::NewListenAddr { address, .. } = event {
                        let full = address.with(Protocol::P2p(peer_id));
                        info!("Local node is listening on {full}");
                    }
                },
                _ = tokio::time::sleep(TokioDuration::from_millis(25)) => {
                    break
                }
            }
        }

        Ok(())
    }

    async fn dial_bootnode(&mut self) {
        match Multiaddr::from_str(&self.boot_addr) {
            Err(err) => {
                warn!("Unable to parse boot addr into Multiaddr: {}", err)
            }
            Ok(address) => {
                match self.swarm.dial(address) {
                    Ok(_) => info!("Dialed bootnode at {}", &self.boot_addr),
                    Err(err) => {
                        warn!("Failed to dial bootnode: {}", err);
                        return;
                    }
                }

                let mut routed = false;

                let duration = TokioDuration::from_secs(1);
                let result = timeout(duration, async {
                    while !routed {
                        match self.swarm.select_next_some().await {
                            SwarmEvent::Behaviour(BehaviorEvent::Kademlia(
                                kad::Event::RoutingUpdated { peer, .. },
                            )) => {
                                info!("Routing table updated with peer: {peer}");
                                let random_peer = PeerId::random();
                                self.swarm
                                    .behaviour_mut()
                                    .kademlia
                                    .get_closest_peers(random_peer);
                            }
                            SwarmEvent::Behaviour(BehaviorEvent::Kademlia(
                                kad::Event::OutboundQueryProgressed {
                                    result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                                    ..
                                },
                            )) => {
                                if ok.peers.is_empty() {
                                    warn!("Find node query yielded no peers");
                                } else {
                                    for discovered_peer in ok.peers {
                                        info!(
                                            "Discovered peer {} from DHT",
                                            discovered_peer.peer_id
                                        );
                                    }
                                    routed = true;
                                }
                            }

                            _ => {}
                        }
                    }
                })
                .await;

                if result.is_err() {
                    warn!("Failed to bootstrap")
                } else {
                    info!("Bootstrap successful!")
                }
            }
        }
    }

    async fn run(
        mut swarm: Swarm<Behavior>,
        mut req_rx: mpsc::Receiver<ClientRequest>,
        gossip_msg_tx: broadcast::Sender<GossipMessage>,
        mut stop_rx: watch::Receiver<()>,
    ) -> Result<(), ()> {
        loop {
            select! {
                Some(request) = req_rx.recv() => Self::handle_client_request(request, &mut swarm),
                event = swarm.select_next_some() => Self::handle_event(event, &gossip_msg_tx, &mut swarm).await,
                _ = stop_rx.changed() => break Ok(()),
            }
        }
    }

    fn handle_client_request(request: ClientRequest, swarm: &mut Swarm<Behavior>) {
        let sender = request.sender;
        let result = match request.payload {
            ClientRequestPayload::Publish { topic, msg } => {
                let tpc = gossipsub::IdentTopic::new(&topic);
                let result = if let Err(err) = swarm.behaviour_mut().gossipsub.publish(tpc, msg) {
                    error!("Publishing Error: {}", err);
                    Err(NetworkError::from(err))
                } else {
                    info!("Successfully published message to {} topic", topic);
                    Ok(ClientResponse::Publish)
                };
                result
            }
            ClientRequestPayload::Subscribe { topic } => {
                let topic = gossipsub::IdentTopic::new(topic);

                let result = if let Err(err) = swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                    error!("Subscription Error: {}", err);
                    Err(NetworkError::from(err))
                } else {
                    info!("Subscribed to topic: {}", topic);
                    Ok(ClientResponse::Subscribe)
                };
                result
            }
            ClientRequestPayload::ConnectedPeers => {
                let peers = swarm.connected_peers().cloned().collect::<Vec<_>>();
                let result = ClientResponse::ConnectedPeers { peers };
                Ok(result)
            }
            ClientRequestPayload::PeerId => {
                let peer_id = *swarm.local_peer_id();
                let result = ClientResponse::PeerId { peer_id };
                Ok(result)
            }
        };

        Self::send_client_response(result, sender);
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
        event: SwarmEvent<BehaviorEvent>,
        gossip_msg_tx: &broadcast::Sender<GossipMessage>,
        swarm: &mut Swarm<Behavior>,
    ) {
        match event {
            SwarmEvent::Behaviour(BehaviorEvent::Gossipsub(gossipsub::Event::Message {
                message,
                propagation_source,
                ..
            })) => {
                info!("Gossip message received from {}", propagation_source);
                match gossip_msg_tx.send(message.data) {
                    Ok(_) => {
                        info!("Gossip message relayed to client");
                    }
                    Err(err) => error!("Error relaying gossip message to client: {}", err),
                }
            }
            SwarmEvent::Behaviour(BehaviorEvent::Gossipsub(gossipsub::Event::Subscribed {
                peer_id,
                topic,
            })) => info!("A remote peer {peer_id} subscribed to a topic: {topic}"),
            SwarmEvent::Behaviour(BehaviorEvent::Identify(identify::Event::Received {
                peer_id: identified_peer,
                info,
                ..
            })) => {
                info!("Identify info received from {identified_peer}");
                for addr in info.listen_addrs {
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&identified_peer, addr);
                }
            }
            _ => {}
        }
        yield_now().await;
    }
}

#[derive(NetworkBehaviour)]
struct Behavior {
    gossipsub: gossipsub::Behaviour,
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

struct ClientRequest {
    payload: ClientRequestPayload,
    sender: oneshot::Sender<Result<ClientResponse, NetworkError>>,
}
pub enum ClientRequestPayload {
    Publish { topic: String, msg: Vec<u8> },
    Subscribe { topic: String },
    ConnectedPeers,
    PeerId,
}

pub enum ClientResponse {
    Publish,
    Subscribe,
    ConnectedPeers { peers: Vec<PeerId> },
    PeerId { peer_id: PeerId },
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

    #[error("{source}")]
    WatchSend {
        #[from]
        source: tokio::sync::watch::error::SendError<()>,
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
