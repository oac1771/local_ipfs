#[cfg(feature = "integration_tests")]
mod tests {
    use integration_tests::utils::{Log, Runner};
    use rand::Rng;
    use server::{
        network::NetworkClient,
        rpc::Module,
        server::{builder::ServerBuilder, ServerConfig},
        state::StateClient,
    };
    use std::sync::{Arc, Mutex};
    use tracing::{instrument, Instrument, Span};
    use tracing_subscriber::{reload::Layer, EnvFilter};

    struct ServerRunner<NC, SC> {
        log_buffer: Arc<Mutex<Vec<u8>>>,
        name: String,
        network_client: NC,
        state_client: SC,
    }

    struct NoNC;
    struct NoSC;

    struct NodeTopology {
        bootnode: ServerRunner<NetworkClient, StateClient>,
        nodes: Vec<ServerRunner<NetworkClient, StateClient>>,
    }

    impl NodeTopology {
        fn new(bootnode: ServerRunner<NetworkClient, StateClient>) -> Self {
            Self {
                bootnode,
                nodes: Vec::new(),
            }
        }

        fn add_node(&mut self, node: ServerRunner<NetworkClient, StateClient>) {
            self.nodes.push(node);
        }

        fn into_nodes(
            self,
        ) -> (
            ServerRunner<NetworkClient, StateClient>,
            Vec<ServerRunner<NetworkClient, StateClient>>,
        ) {
            (self.bootnode, self.nodes)
        }
    }

    impl ServerRunner<NoNC, NoSC> {
        fn new(log_buffer: Arc<Mutex<Vec<u8>>>, name: impl Into<String>) -> Self {
            Self {
                log_buffer,
                name: name.into(),
                network_client: NoNC,
                state_client: NoSC,
            }
        }

        #[instrument(skip_all, fields(label = %self.name))]
        async fn start(
            self,
            port: impl Into<String>,
            network_port: impl Into<String>,
            is_boot_node: bool,
            boot_node_addr: impl Into<String>,
        ) -> ServerRunner<NetworkClient, StateClient> {
            let config = ServerConfig {
                port: port.into(),
                network_port: network_port.into(),
                ip: "0.0.0.0".into(),
                modules: vec![Module::Util],
                is_boot_node,
                boot_node_addr: boot_node_addr.into(),
            };
            let (_, handle) = Layer::new(EnvFilter::default());
            let server = ServerBuilder::new(config).build(handle).await.unwrap();

            let network_client = server.network_client().clone();
            let state_client = server.state_client().clone();

            let span = Span::current();
            let _ = tokio::spawn(server.run().instrument(span));

            ServerRunner {
                log_buffer: self.log_buffer,
                name: self.name,
                network_client: network_client,
                state_client: state_client,
            }
        }
    }

    impl ServerRunner<NetworkClient, StateClient> {
        fn network_client(&self) -> &NetworkClient {
            &self.network_client
        }

        fn state_client(&self) -> &StateClient {
            &self.state_client
        }
    }

    impl<NC, SC> Runner for ServerRunner<NC, SC> {
        fn log_buffer(&self) -> Arc<Mutex<Vec<u8>>> {
            self.log_buffer.clone()
        }

        fn log_filter(&self, log: &Log) -> bool {
            log.label() == self.name
        }
    }

    async fn setup_test_topolgy(
        additional_nodes: usize,
        log_buffer: Arc<Mutex<Vec<u8>>>,
    ) -> NodeTopology {
        let mut rng = rand::thread_rng();
        let port_range = 49152..=65535;
        let bootnode_port = format!("{}", rng.gen_range(port_range.clone()));
        let bootnode_network_port = format!("{}", rng.gen_range(port_range.clone()));

        let bootnode = ServerRunner::new(log_buffer.clone(), "bootnode")
            .start(bootnode_port, &bootnode_network_port, true, "")
            .await;

        let bootnode_peer_id = bootnode.network_client.get_peer_id().await.unwrap();
        let boot_node_addr = format!(
            "/ip4/127.0.0.1/tcp/{}/p2p/{}",
            bootnode_network_port, bootnode_peer_id
        );

        let node_port = format!("{}", rng.gen_range(port_range.clone()));
        let node_network_port = format!("{}", rng.gen_range(port_range.clone()));
        let node = ServerRunner::new(log_buffer.clone(), "node_1")
            .start(node_port, node_network_port, false, &boot_node_addr)
            .await;

        let mut node_topology = NodeTopology::new(bootnode);
        node_topology.add_node(node);

        for index in 1..=additional_nodes {
            let node_port = format!("{}", rng.gen_range(port_range.clone()));
            let node_network_port = format!("{}", rng.gen_range(port_range.clone()));
            let node = ServerRunner::new(log_buffer.clone(), format!("node_{}", index + 1))
                .start(node_port, node_network_port, false, &boot_node_addr)
                .await;
            node_topology.add_node(node);
        }

        node_topology
    }

    #[test_macro::test]
    async fn bootstrap_to_bootnode_succeeds(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let node_topology = setup_test_topolgy(1, log_buffer).await;
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

    // #[test_macro::test]
    // async fn gossip_message_to_peers(log_buffer: Arc<Mutex<Vec<u8>>>) {
    //     let bootnode_port = "9998";
    //     let node_1_port = "8888";
    //     let node_2_port = "9999";
    //     let bootnode_network_port = "58763";

    //     let bootnode = ServerRunner::new(log_buffer.clone(), "bootnode");
    //     let node_1 = ServerRunner::new(log_buffer.clone(), "node_1");
    //     let node_2 = ServerRunner::new(log_buffer.clone(), "node_2");

    //     let (bootnode_network_client, _) = bootnode
    //         .start(bootnode_port, bootnode_network_port, true, "")
    //         .await;
    //     let bootnode_peer_id = bootnode_network_client.get_peer_id().await.unwrap();
    //     let bootnode_addr = format!(
    //         "/ip4/127.0.0.1/tcp/{}/p2p/{}",
    //         bootnode_network_port, bootnode_peer_id
    //     );

    //     let (node_1_network_client, _) = node_1
    //         .start(node_1_port, "0", false, bootnode_addr.clone())
    //         .await;
    //     let (node_2_network_client, _) = node_2.start(node_2_port, "0", false, bootnode_addr).await;

    //     let node_1_peer_id = node_1_network_client.get_peer_id().await.unwrap();
    //     let node_2_peer_id = node_2_network_client.get_peer_id().await.unwrap();
    // }
}
