use tokio::sync::Mutex;
use tonic::{
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    transport::Channel,
    Request, Response, Status,
};

use crate::{
    common::{DeploymentIdRequest, TemplateIdRequest},
    deployment::deployment_client::DeploymentClient,
    DeploymentMessage, DeploymentTrait, ListDeploymentsRequest, ListDeploymentsResponse,
    OperationId, WorkloadIdRequest,
};

use super::interceptor::ClientInterceptor;

pub struct WrappedDeploymentClient {
    inner: Mutex<DeploymentClient<InterceptedService<Channel, ClientInterceptor>>>,
}

impl WrappedDeploymentClient {
    pub fn new(channel: Channel, token: MetadataValue<Ascii>) -> Self {
        let inner = DeploymentClient::with_interceptor(channel, ClientInterceptor { token });
        let inner = Mutex::new(inner);

        WrappedDeploymentClient { inner }
    }
}

#[tonic::async_trait]
impl DeploymentTrait for WrappedDeploymentClient {
    async fn upsert(
        &self,
        request: Request<DeploymentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.upsert(request).await
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
        let test = inner.get_by_id(request).await;

        println!("test: {:?}", test);

        test
    }

    async fn get_by_template_id(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.get_by_template_id(request).await
    }

    async fn get_by_workload_id(
        &self,
        request: Request<WorkloadIdRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.get_by_workload_id(request).await
    }

    async fn list(
        &self,
        request: Request<ListDeploymentsRequest>,
    ) -> Result<Response<ListDeploymentsResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.list(request).await
    }
}
