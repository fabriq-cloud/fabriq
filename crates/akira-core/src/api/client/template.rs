use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{
    codegen::InterceptedService,
    metadata::{Ascii, MetadataValue},
    transport::Channel,
    Request, Response, Status,
};

use crate::{
    common::TemplateIdRequest, template::template_client::TemplateClient, DeleteTemplateRequest,
    ListTemplatesRequest, ListTemplatesResponse, OperationId, TemplateMessage, TemplateTrait,
};

use super::interceptor::ClientInterceptor;

pub struct WrappedTemplateClient {
    inner: Arc<Mutex<TemplateClient<InterceptedService<Channel, ClientInterceptor>>>>,
}

impl WrappedTemplateClient {
    pub fn new(channel: Channel, token: MetadataValue<Ascii>) -> Self {
        let inner = TemplateClient::with_interceptor(channel, ClientInterceptor { token });
        let inner = Arc::new(Mutex::new(inner));

        WrappedTemplateClient { inner }
    }
}

#[tonic::async_trait]
impl TemplateTrait for WrappedTemplateClient {
    async fn create(
        &self,
        request: Request<TemplateMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.create(request).await
    }

    async fn delete(
        &self,
        request: Request<DeleteTemplateRequest>,
    ) -> Result<Response<OperationId>, Status> {
        let mut inner = self.inner.lock().await;
        inner.delete(request).await
    }

    async fn get_by_id(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<TemplateMessage>, Status> {
        let mut inner = self.inner.lock().await;
        inner.get_by_id(request).await
    }

    async fn list(
        &self,
        request: Request<ListTemplatesRequest>,
    ) -> Result<Response<ListTemplatesResponse>, Status> {
        let mut inner = self.inner.lock().await;
        inner.list(request).await
    }
}
