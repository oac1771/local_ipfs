#[cfg(test)]
mod tests {
    use futures::FutureExt;
    use integration_tests::utils::{Log, Runner};
    use jsonrpsee::{core::client::Client, ws_client::WsClientBuilder};

    use rand::Rng;
    use server::{
        api::ipfs::IpfsClient,
        network::{GossipCallBackFn, NetworkBuilder, NetworkClient},
        rpc::{ipfs::GossipMessage, Module},
        server::{builder::ServerBuilder, Server, ServerConfig},
        state::State,
    };
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };
    use tracing::{info, instrument, Instrument, Span};
    use tracing_subscriber::{reload::Layer, EnvFilter};

    struct ServerRunnerBuilder {
        name: String,
        server_config: ServerConfig,
        log_buffer: Arc<Mutex<Vec<u8>>>,
    }

    struct ServerRunner {
        log_buffer: Arc<Mutex<Vec<u8>>>,
        name: String,
        server_client: Client,
        network_client: NetworkClient,
    }

    struct NodeTopology {
        bootnode: ServerRunner,
        nodes: Vec<ServerRunner>,
    }

    impl NodeTopology {
        fn new(bootnode: ServerRunner) -> Self {
            Self {
                bootnode,
                nodes: Vec::new(),
            }
        }

        fn add_node(&mut self, node: ServerRunner) {
            self.nodes.push(node);
        }

        fn into_nodes(self) -> (ServerRunner, Vec<ServerRunner>) {
            (self.bootnode, self.nodes)
        }
    }

    impl ServerRunnerBuilder {
        async fn new(
            log_buffer: Arc<Mutex<Vec<u8>>>,
            name: impl Into<String>,
            port: impl Into<String>,
            network_port: impl Into<String>,
            is_boot_node: bool,
            boot_node_addr: impl Into<String>,
            topic: impl Into<String>,
        ) -> Self {
            let server_config = ServerConfig {
                port: port.into(),
                network_port: network_port.into(),
                ip: "0.0.0.0".into(),
                modules: vec![Module::Util, Module::Ipfs],
                is_boot_node,
                boot_node_addr: boot_node_addr.into(),
                topic: topic.into(),
                ipfs_base_url: "".into(),
                push_gateway_url: "".into(),
            };

            Self {
                server_config,
                log_buffer,
                name: name.into(),
            }
        }

        #[instrument(skip_all, fields(label = %self.name))]
        async fn start(self) -> ServerRunner {
            let span = Span::current();
            let server_port = self.server_config.port.clone();

            let (_, handle) = Layer::new(EnvFilter::default());
            let network = NetworkBuilder::new()
                .with_port(&self.server_config.network_port)
                .with_is_boot_node(self.server_config.is_boot_node)
                .with_boot_addr(&self.server_config.boot_node_addr)
                .with_topic(&self.server_config.topic)
                .build()
                .unwrap();
            let state = State::new();

            let gossip_callback_fns =
                Self::build_network_gossip_callback_fns(&self.server_config.modules);

            let network_client = network.start(gossip_callback_fns).await.unwrap();
            let state_client = state.start();

            let server = ServerBuilder::new(self.server_config)
                .build(handle, network_client.clone(), state_client.clone())
                .await
                .unwrap();

            let _ = tokio::spawn({
                let network_client = network_client.clone();
                let state_client = state_client.clone();

                async move {
                    let server_handle = server.run().instrument(span).await.unwrap();
                    Server::wait(&network_client, &state_client, server_handle).await;
                }
            });

            let server_url = format!("ws://localhost:{}", server_port);

            let server_client = tokio::time::timeout(tokio::time::Duration::from_secs(1), async {
                let client = loop {
                    match WsClientBuilder::default()
                        .request_timeout(Duration::from_millis(100))
                        .build(&server_url)
                        .await
                    {
                        Ok(server_client) => break server_client,
                        Err(_) => tokio::time::sleep(tokio::time::Duration::from_millis(10)).await,
                    }
                };
                client
            })
            .await
            .expect("Timedout waiting for server client");

            ServerRunner {
                log_buffer: self.log_buffer,
                name: self.name,
                server_client,
                network_client,
            }
        }

        fn build_network_gossip_callback_fns(modules: &[Module]) -> Vec<GossipCallBackFn> {
            modules
                .iter()
                .filter_map(|m| {
                    if let Module::Ipfs = m {
                        let callback_fn: GossipCallBackFn = Box::new({
                            move |msg: &[u8]| {
                                async move {
                                    if let Ok(GossipMessage::AddFile { hash: _ }) =
                                        serde_json::from_slice::<GossipMessage>(msg)
                                    {
                                        info!("Processing add file gossip message");
                                    }
                                }
                                .boxed()
                            }
                        });

                        Some(callback_fn)
                    } else {
                        None
                    }
                })
                .collect::<Vec<GossipCallBackFn>>()
        }
    }

    impl ServerRunner {
        fn network_client(&self) -> &NetworkClient {
            &self.network_client
        }
    }

    impl Runner for ServerRunner {
        fn log_buffer(&self) -> Arc<Mutex<Vec<u8>>> {
            self.log_buffer.clone()
        }

        fn log_filter(&self, log: &Log) -> bool {
            log.label() == self.name()
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    async fn setup_test_topolgy(
        additional_nodes: usize,
        log_buffer: Arc<Mutex<Vec<u8>>>,
        topic: impl Into<String> + std::marker::Copy,
    ) -> NodeTopology {
        let mut rng = rand::thread_rng();
        let port_range = 49152..=65535;
        let bootnode_port = format!("{}", rng.gen_range(port_range.clone()));
        let bootnode_network_port = format!("{}", rng.gen_range(port_range.clone()));

        let bootnode = ServerRunnerBuilder::new(
            log_buffer.clone(),
            "bootnode",
            bootnode_port,
            &bootnode_network_port,
            true,
            "",
            topic,
        )
        .await
        .start()
        .await;

        let bootnode_peer_id = bootnode.network_client.get_peer_id().await.unwrap();
        let boot_node_addr = format!(
            "/ip4/127.0.0.1/tcp/{}/p2p/{}",
            bootnode_network_port, bootnode_peer_id
        );

        let node_port = format!("{}", rng.gen_range(port_range.clone()));
        let node_network_port = format!("{}", rng.gen_range(port_range.clone()));
        let node = ServerRunnerBuilder::new(
            log_buffer.clone(),
            "node_1",
            node_port,
            node_network_port,
            false,
            &boot_node_addr,
            topic,
        )
        .await
        .start()
        .await;

        let mut node_topology = NodeTopology::new(bootnode);
        node_topology.add_node(node);

        for index in 1..=additional_nodes {
            let node_port = format!("{}", rng.gen_range(port_range.clone()));
            let node_network_port = format!("{}", rng.gen_range(port_range.clone()));
            let node = ServerRunnerBuilder::new(
                log_buffer.clone(),
                format!("node_{}", index + 1),
                node_port,
                node_network_port,
                false,
                &boot_node_addr,
                topic,
            )
            .await
            .start()
            .await;

            node_topology.add_node(node);
        }

        node_topology
    }

    #[test_macro::test]
    async fn bootstrap_to_bootnode_succeeds(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let node_topology = setup_test_topolgy(1, log_buffer, "topic").await;
        let (bootnode, nodes) = node_topology.into_nodes();

        let (node_1, node_2) = match &nodes[..] {
            [first, second, ..] => (first, second),
            _ => panic!("Not enough peers"),
        };

        node_1
            .assert_info_log_contains("Starting Server on: 0.0.0.0:")
            .await;
        node_2
            .assert_info_log_contains("Starting Server on: 0.0.0.0:")
            .await;
        bootnode
            .assert_info_log_contains("Starting Server on: 0.0.0.0:")
            .await;
        node_1.assert_info_log_entry("Bootstrap successful!").await;

        node_2.assert_info_log_entry("Bootstrap successful!").await;

        let bootnode_peer_id = bootnode.network_client().get_peer_id().await.unwrap();
        let node_1_peer_id = node_1.network_client().get_peer_id().await.unwrap();
        let node_2_peer_id = node_2.network_client().get_peer_id().await.unwrap();

        let mut node_1_peers = node_1.network_client().get_connected_peers().await.unwrap();
        let mut node_2_peers = node_2.network_client().get_connected_peers().await.unwrap();

        assert_eq!(
            vec![node_1_peer_id, bootnode_peer_id].sort(),
            node_2_peers.sort()
        );
        assert_eq!(
            vec![node_2_peer_id, bootnode_peer_id].sort(),
            node_1_peers.sort()
        );
    }

    #[test_macro::test]
    async fn gossip_message_to_peers(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let topic = "gossip_topic";
        let msg = b"hello world".to_vec();
        let node_topology = setup_test_topolgy(1, log_buffer, topic).await;
        let (_, nodes) = node_topology.into_nodes();

        let (node_1, node_2) = match &nodes[..] {
            [first, second, ..] => (first, second),
            _ => panic!("Not enough peers"),
        };
        let mut gossip_receiver = node_2.network_client().gossip_receiver().await;

        let node_1_peer_id = node_1.network_client().get_peer_id().await.unwrap();

        node_1
            .assert_info_log_entry(&format!("Subscribed to topic: {}", topic))
            .await;
        node_2
            .assert_info_log_entry(&format!("Subscribed to topic: {}", topic))
            .await;

        node_1.network_client().publish(msg.clone()).await.unwrap();

        node_1
            .assert_info_log_entry(&format!(
                "Successfully published message to {} topic",
                topic
            ))
            .await;
        node_2
            .assert_info_log_entry(&format!("Gossip message received from {}", node_1_peer_id))
            .await;
        node_2
            .assert_info_log_entry("Gossip message relayed to client")
            .await;

        let gossip_msg = gossip_receiver.recv().await.unwrap();

        assert_eq!(gossip_msg, msg.to_vec());
    }

    #[test_macro::test]
    async fn gossip_ipfs_add_file_to_peers(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let topic = "gossip_topic";
        let data = vec![1, 2, 3, 4];

        let node_topology = setup_test_topolgy(1, log_buffer, topic).await;
        let (_, nodes) = node_topology.into_nodes();

        let (node_1, node_2) = match &nodes[..] {
            [first, second, ..] => (first, second),
            _ => panic!("Not enough peers"),
        };

        let node_1_peer_id = node_1.network_client().get_peer_id().await.unwrap();

        node_1
            .assert_info_log_entry(&format!("Subscribed to topic: {}", topic))
            .await;
        node_2
            .assert_info_log_entry(&format!("Subscribed to topic: {}", topic))
            .await;

        node_1.server_client.add(data).await.unwrap();

        node_1
            .assert_info_log_entry(&format!(
                "Successfully published message to {} topic",
                topic
            ))
            .await;

        node_2
            .assert_info_log_contains(&format!("Gossip message received from {}", node_1_peer_id))
            .await;
        node_2
            .assert_info_log_entry("Gossip message relayed to client")
            .await;
        node_2
            .assert_info_log_entry("Processing add file gossip message")
            .await;
    }
}
