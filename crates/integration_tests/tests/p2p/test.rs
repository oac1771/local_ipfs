#[cfg(feature = "integration_tests")]
mod tests {
    use integration_tests::utils::Runner;
    use server::{
        network::NetworkClient,
        rpc::Module,
        server::{builder::ServerBuilder, ServerConfig},
        state::StateClient,
    };
    use std::sync::{Arc, Mutex};
    use tracing::instrument;
    use tracing_subscriber::{reload::Layer, EnvFilter};

    struct ServerRunner {
        log_buffer: Arc<Mutex<Vec<u8>>>,
        name: String,
    }

    impl ServerRunner {
        fn new(log_buffer: Arc<Mutex<Vec<u8>>>, name: impl Into<String>) -> Self {
            Self {
                log_buffer,
                name: name.into(),
            }
        }

        #[instrument(skip(self, port, boot_node_addr, network_port), fields(label = %self.name))]
        async fn start(
            &self,
            port: impl Into<String>,
            network_port: impl Into<String>,
            is_boot_node: bool,
            boot_node_addr: impl Into<String>,
        ) -> (NetworkClient, StateClient) {
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

            tokio::spawn(server.run());

            (network_client, state_client)
        }
    }

    impl Runner for ServerRunner {
        fn log_buffer(&self) -> Arc<Mutex<Vec<u8>>> {
            self.log_buffer.clone()
        }
    }

    #[test_macro::test]
    async fn bootstrap_to_bootnode_succeeds(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let bootnode_port = "9998";
        let bootnode_network_port = "58763";
        let bootnode = ServerRunner::new(log_buffer.clone(), "bootnode");
        let node_1 = ServerRunner::new(log_buffer.clone(), "node_1");
        let node_2 = ServerRunner::new(log_buffer.clone(), "node_2");

        let (bootnode_network_client, _) = bootnode
            .start(bootnode_port, bootnode_network_port, true, "")
            .await;
        let bootnode_peer_id = bootnode_network_client.get_peer_id().await.unwrap();
        let bootnode_addr = format!(
            "/ip4/127.0.0.1/tcp/{}/p2p/{}",
            bootnode_network_port, bootnode_peer_id
        );

        let (node_1_network_client, _) = node_1
            .start("8888", "0", false, bootnode_addr.clone())
            .await;
        let (node_2_network_client, _) = node_2.start("9999", "0", false, bootnode_addr).await;

        let node_1_peer_id = node_1_network_client.get_peer_id().await.unwrap();
        let node_2_peer_id = node_2_network_client.get_peer_id().await.unwrap();

        node_1
            .assert_info_log_entry("Starting Server on: 0.0.0.0:8888")
            .await;
        node_2
            .assert_info_log_entry("Starting Server on: 0.0.0.0:9999")
            .await;
        bootnode
            .assert_info_log_entry(&format!("Starting Server on: 0.0.0.0:{}", bootnode_port))
            .await;

        node_1
            .assert_info_log_entry(&format!(
                "Routing table updated with peer: {}",
                bootnode_peer_id
            ))
            .await;
        node_1.assert_info_log_entry("Bootstrap successful!").await;

        node_2
            .assert_info_log_entry(&format!(
                "Routing table updated with peer: {}",
                bootnode_peer_id
            ))
            .await;
        node_2.assert_info_log_entry("Bootstrap successful!").await;

        bootnode
            .assert_info_log_entry(&format!("Identify info received from {}", node_1_peer_id))
            .await;
        bootnode
            .assert_info_log_entry(&format!("Identify info received from {}", node_2_peer_id))
            .await;

        let mut node_1_peers = node_1_network_client.get_connected_peers().await.unwrap();
        let mut node_2_peers = node_2_network_client.get_connected_peers().await.unwrap();

        assert_eq!(
            vec![node_1_peer_id, bootnode_peer_id].sort(),
            node_2_peers.sort()
        );
        assert_eq!(
            vec![node_2_peer_id, bootnode_peer_id].sort(),
            node_1_peers.sort()
        );
    }
}
