use fabriq_core::{
    common::TemplateIdRequest, ListTemplatesRequest, ListTemplatesResponse, OperationId,
    TemplateMessage, TemplateTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Template;
use crate::services::TemplateService;

#[derive(Debug)]
pub struct GrpcTemplateService {
    service: Arc<TemplateService>,
}
impl GrpcTemplateService {
    pub fn new(service: Arc<TemplateService>) -> Self {
        GrpcTemplateService { service }
    }
}

#[tonic::async_trait]
impl TemplateTrait for GrpcTemplateService {
    #[tracing::instrument(name = "grpc::template::upsert")]
    async fn upsert(
        &self,
        request: Request<TemplateMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_template: Template = request.into_inner().into();

        let operation_id = match self.service.upsert(&new_template, None).await {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("creating template failed with {}", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::target::delete")]
    async fn delete(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Check that no workloads are currently still using template
        // Query workload service for workloads by template_id

        let operation_id = match self
            .service
            .delete(&request.into_inner().template_id, None)
            .await
        {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("deleting workspace failed with {}", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    #[tracing::instrument(name = "grpc::target::get_by_id")]
    async fn get_by_id(
        &self,
        request: Request<TemplateIdRequest>,
    ) -> Result<Response<TemplateMessage>, Status> {
        let template_id = request.into_inner().template_id;
        let template = match self.service.get_by_id(&template_id).await {
            Ok(template) => template,
            Err(err) => {
                tracing::error!("get target with id {}: failed: {}", template_id, err);
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("get target with id {}: failed", &template_id),
                ));
            }
        };

        let template = match template {
            Some(template) => template,
            None => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("get template with id {}: not found", &template_id),
                ))
            }
        };

        let template_message: TemplateMessage = template.into();

        Ok(Response::new(template_message))
    }

    #[tracing::instrument(name = "grpc::target::list")]
    async fn list(
        &self,
        _request: Request<ListTemplatesRequest>,
    ) -> Result<Response<ListTemplatesResponse>, Status> {
        let templates = match self.service.list().await {
            Ok(templates) => templates,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing templates failed with {}", err),
                ))
            }
        };

        let template_messages = templates
            .iter()
            .map(|template| TemplateMessage {
                id: template.id.clone(),
                repository: template.repository.clone(),
                git_ref: template.git_ref.clone(),
                path: template.path.clone(),
            })
            .collect();

        let response = ListTemplatesResponse {
            templates: template_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use fabriq_core::{
        common::TemplateIdRequest, test::get_template_fixture, EventStream, ListTemplatesRequest,
        TemplateTrait,
    };
    use fabriq_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::GrpcTemplateService;

    use crate::models::Template;
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::TemplateService;

    #[tokio::test]
    async fn test_create_list_template() -> anyhow::Result<()> {
        let template_persistence = Box::new(MemoryPersistence::<Template>::default());
        let event_stream = Arc::new(MemoryEventStream::new().unwrap()) as Arc<dyn EventStream>;

        let template_service = Arc::new(TemplateService {
            persistence: template_persistence,
            event_stream,
        });

        let template_grpc_service = GrpcTemplateService::new(Arc::clone(&template_service));

        let template = get_template_fixture(None);

        let request = Request::new(template.clone());

        let create_response = template_grpc_service
            .upsert(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(create_response.id.len(), 36);

        let request = Request::new(ListTemplatesRequest {});

        let list_response = template_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(list_response.templates.len(), 1);

        let request = Request::new(TemplateIdRequest {
            template_id: template.id.clone(),
        });

        let get_by_id_response = template_grpc_service.get_by_id(request).await.unwrap();

        assert_eq!(get_by_id_response.into_inner().id, template.id);

        let request = Request::new(TemplateIdRequest {
            template_id: template.id,
        });

        let delete_response = template_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(delete_response.id.len(), 36);

        Ok(())
    }
}
