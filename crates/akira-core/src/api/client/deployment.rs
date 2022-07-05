use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    transport::Channel,
    Request, Response, Status,
};

use crate::{
    common::DeploymentIdRequest, deployment::deployment_client::DeploymentClient,
    DeploymentMessage, DeploymentTrait, ListDeploymentsRequest, ListDeploymentsResponse,
    OperationId,
};

use super::interceptor::ClientInterceptor;

pub struct WrappedDeploymentClient {
    inner: Arc<Mutex<DeploymentClient<InterceptedService<Channel, ClientInterceptor>>>>,
}

impl WrappedDeploymentClient {
    pub fn new(channel: Channel, token: MetadataValue<Ascii>) -> Self {
        let inner = DeploymentClient::with_interceptor(channel, ClientInterceptor { token });
        let inner = Arc::new(Mutex::new(inner));

        WrappedDeploymentClient { inner }
    }
}

#[tonic::async_trait]
impl DeploymentTrait for WrappedDeploymentClient {
    async fn create(
        &self,
        request: Request<DeploymentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.create(request).await
    }

    async fn delete(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.delete(request).await
    }

    async fn get_by_id(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<DeploymentMessage>, Status> {
        let mut inner = self.inner.lock().await;
        inner.get_by_id(request).await
    }

    async fn list(
        &self,
        request: Request<ListDeploymentsRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.list(request).await
    }
}
