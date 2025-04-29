#[cfg(feature = "integration_tests")]
mod tests {
    use integration_tests::utils::{Log, Runner};
    use libp2p::futures::StreamExt;
    use rand::{
        distributions::Alphanumeric,
        {thread_rng, Rng},
    };
    use std::sync::{Arc, Mutex};
    use tokio::{
        select,
        task::JoinHandle,
        time::{sleep, Duration},
    };
    use tracing::instrument;

    struct NodeRunner<'a> {
        log_buffer: Arc<Mutex<Vec<u8>>>,
        name: &'a str,
    }

    impl<'a> NodeRunner<'a> {
        fn new(log_buffer: Arc<Mutex<Vec<u8>>>, name: &'a str) -> Self {
            Self { log_buffer, name }
        }

        #[instrument(skip(self), fields(label = %self.name))]
        fn start(&self) -> (JoinHandle<Result<(), NetworkError>>, NodeClient) {
            let node = NodeBuilder::build().unwrap();
            let (handle, node_client) = node.start().unwrap();

            (handle, node_client)
        }
    }

    impl<'a> Runner for NodeRunner<'a> {
        fn log_filter(&self, log: &Log) -> bool {
            log.spans()
                .into_iter()
                .any(|val| val.to_string().contains(self.name))
        }

        fn log_buffer(&self) -> Arc<Mutex<Vec<u8>>> {
            self.log_buffer.clone()
        }
    }

    async fn wait_for_gossip_nodes(client: &NodeClient, topic: &str) -> bool {
        while let None = client.get_gossip_nodes(topic).await.unwrap().next() {}
        true
    }

    #[test_macro::test]
    async fn mdns_and_gossip_discovery_success(log_buffer: Arc<Mutex<Vec<u8>>>) {
        let topic: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(5)
            .map(|val| char::from(val))
            .collect();

        let node_1 = NodeRunner::new(log_buffer.clone(), "node_1");
        let node_2 = NodeRunner::new(log_buffer.clone(), "node_2");

        let (_, client_1) = node_1.start();
        let (_, client_2) = node_2.start();

        client_1.subscribe(&topic).await.unwrap();
        client_2.subscribe(&topic).await.unwrap();

        let network_id_1 = client_1.get_local_network_id().await.unwrap();
        let network_id_2 = client_2.get_local_network_id().await.unwrap();

        node_1
            .assert_info_log_entry(&format!("mDNS discovered a new peer: {}", network_id_2))
            .await;
        node_2
            .assert_info_log_entry(&format!("mDNS discovered a new peer: {}", network_id_1))
            .await;

        node_1
            .assert_info_log_entry(&format!("A remote subscribed to a topic: {}", topic))
            .await;
        node_2
            .assert_info_log_entry(&format!("A remote subscribed to a topic: {}", topic))
            .await;

        let mut gossip_nodes_1 = client_1.get_gossip_nodes(&topic).await.unwrap();
        let mut gossip_nodes_2 = client_2.get_gossip_nodes(&topic).await.unwrap();

        assert_eq!(gossip_nodes_1.next(), Some(network_id_2));
        assert_eq!(gossip_nodes_2.next(), Some(network_id_1));
    }
}