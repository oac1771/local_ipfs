use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io,
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
use tracing::{error, info};

type GossipMessage = Vec<u8>;
pub struct NetworkBuilder;

pub struct Network {
    swarm: Swarm<Behavior>,
}

#[derive(Clone)]
pub struct NetworkClient {
    req_tx: mpsc::Sender<ClientRequest>,
    gossip_msg_tx: broadcast::Sender<GossipMessage>,
    stop_tx: watch::Sender<()>,
}

impl NetworkBuilder {
    pub fn build() -> Result<Network, NetworkError> {
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

        Ok(Network { swarm })
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

    pub async fn _gossip_receiver(&self) -> broadcast::Receiver<GossipMessage> {
        self.gossip_msg_tx.subscribe()
    }
}

impl Network {
    pub async fn start(mut self) -> Result<NetworkClient, NetworkError> {
        let (req_tx, req_rx) = mpsc::channel::<ClientRequest>(100);
        let (gossip_msg_tx, _) = broadcast::channel::<GossipMessage>(100);
        let (stop_tx, stop_rx) = watch::channel(());

        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        self.get_listener_addresses().await?;

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

    #[error("Error: {0}")]
    Behavior(String),

    #[error("Error: {0}")]
    EmptyListeners(String),
}
