#[cfg(feature = "integration_tests")]
mod tests {
    use integration_tests::utils::{Log, Runner};
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

        #[instrument(skip(self, port, boot_node_addr), fields(label = %self.name))]
        async fn start(
            &self,
            port: impl Into<String>,
            is_boot_node: bool,
            boot_node_addr: impl Into<String>,
        ) -> (NetworkClient, StateClient) {
            let config = ServerConfig {
                port: port.into(),
                network_port: "0".into(),
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
        fn log_filter(&self, log: &Log) -> bool {
            log.spans()
                .iter()
                .any(|val| val.to_string().contains(&self.name))
        }

        fn log_buffer(&self) -> Arc<Mutex<Vec<u8>>> {
            self.log_buffer.clone()
        }
    }

    #[test_macro::test]
    async fn bootstrap_to_bootnode_succeeds(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let _node_1 = ServerRunner::new(log_buffer.clone(), "node_1");
        let _node_2 = ServerRunner::new(log_buffer.clone(), "node_2");

        // let (_, client_1) = node_1.start();
        // let (_, client_2) = node_2.start();
    }
}
