use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    transport::Channel,
    Request, Response, Status,
};

use crate::{
    config::config_client::ConfigClient, ConfigIdRequest, ConfigMessage, ConfigTrait, OperationId,
    QueryConfigRequest, QueryConfigResponse,
};

use super::interceptor::ClientInterceptor;

pub struct WrappedConfigClient {
    inner: Arc<Mutex<ConfigClient<InterceptedService<Channel, ClientInterceptor>>>>,
}

impl WrappedConfigClient {
    pub fn new(channel: Channel, token: MetadataValue<Ascii>) -> Self {
        let inner = ConfigClient::with_interceptor(channel, ClientInterceptor { token });
        let inner = Arc::new(Mutex::new(inner));

        WrappedConfigClient { inner }
    }
}

#[tonic::async_trait]
impl ConfigTrait for WrappedConfigClient {
    async fn create(
        &self,
        request: Request<ConfigMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.create(request).await
    }

    async fn delete(
        &self,
        request: Request<ConfigIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.delete(request).await
    }

    async fn query(
        &self,
        request: Request<QueryConfigRequest>,
    ) -> Result<Response<QueryConfigResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.query(request).await
    }
}
