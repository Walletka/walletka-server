use anyhow::Result;
use proto::node_client::NodeClient;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};
use tonic::transport::Channel;
use tower::ServiceBuilder;

pub mod proto {
    tonic::include_proto!("node_api_service");
}

pub struct LightningNodeGrpcClient {
    pub client: NodeClient<Channel>,
    address: String,
    keep_alive: bool,
}

impl Deref for LightningNodeGrpcClient {
    type Target = NodeClient<Channel>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for LightningNodeGrpcClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

impl LightningNodeGrpcClient {
    pub async fn new(address: String, keep_alive: bool) -> Result<Self> {
        let client = Self::get_client(address.clone(), keep_alive).await.unwrap();

        Ok(Self {
            client,
            address,
            keep_alive,
        })
    }

    pub async fn reconnect(&mut self) -> Result<()> {
        self.client = Self::get_client(self.address.clone(), self.keep_alive).await?;
        Ok(())
    }

    async fn get_client(address: String, keep_alive: bool) -> Result<NodeClient<Channel>> {
        let channel = tonic::transport::Channel::from_shared(address)?
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
}
