use anyhow::Result;
use proto::node_client::NodeClient;
use std::time::Duration;
use tonic::transport::Channel;
use tower::ServiceBuilder;

pub mod proto {
    tonic::include_proto!("node_api_service");
}

pub async fn get_lightning_node_client(
    address: String,
    keep_alive: bool,
) -> Result<NodeClient<Channel>> {
    let channel = Channel::from_shared(address)?
        .connect_timeout(Duration::from_secs(10)) // Set connection timeout
        .keep_alive_while_idle(keep_alive) // Set keep-alive
        .connect()
        .await?;

    let channel = ServiceBuilder::new()
        // Interceptors can be also be applied as middleware
        //.layer(tonic::service::interceptor(intercept))
        //.layer_fn(AuthSvc::new)
        .service(channel);

    Ok(NodeClient::new(channel))
}
