use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    transport::Channel,
    Request, Response, Status,
};

use crate::{
    common::{TemplateIdRequest, WorkloadIdRequest},
    workload::workload_client::WorkloadClient,
    ListWorkloadsRequest, ListWorkloadsResponse, OperationId, WorkloadMessage, WorkloadTrait,
};

use super::interceptor::ClientInterceptor;

pub struct WrappedWorkloadClient {
    inner: Arc<Mutex<WorkloadClient<InterceptedService<Channel, ClientInterceptor>>>>,
}

impl WrappedWorkloadClient {
    pub fn new(channel: Channel, token: MetadataValue<Ascii>) -> Self {
        let inner = WorkloadClient::with_interceptor(channel, ClientInterceptor { token });
        let inner = Arc::new(Mutex::new(inner));

        WrappedWorkloadClient { inner }
    }
}

#[tonic::async_trait]
impl WorkloadTrait for WrappedWorkloadClient {
    async fn upsert(
        &self,
        request: Request<WorkloadMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.upsert(request).await
    }

    async fn delete(
        &self,
        request: Request<WorkloadIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.delete(request).await
    }

    async fn get_by_id(
        &self,
        request: Request<WorkloadIdRequest>,
    ) -> Result<Response<WorkloadMessage>, Status> {
        let mut inner = self.inner.lock().await;
        inner.get_by_id(request).await
    }

    async fn get_by_template_id(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.get_by_template_id(request).await
    }

    async fn list(
        &self,
        request: Request<ListWorkloadsRequest>,
    ) -> Result<Response<ListWorkloadsResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.list(request).await
    }
}
