use akira_core::{
    DeleteTemplateRequest, ListTemplatesRequest, ListTemplatesResponse, OperationId,
    TemplateMessage, TemplateTrait,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Template;
use crate::services::TemplateService;

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
    async fn create(
        &self,
        request: Request<TemplateMessage>,
    ) -> Result<Response<OperationId>, Status> {
        let new_template: Template = request.into_inner().into();

        let operation_id = match self.service.create(&new_template, None) {
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

    async fn delete(
        &self,
        request: Request<DeleteTemplateRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Check that no workloads are currently still using template
        // Query workload service for workloads by template_id

        let operation_id = match self.service.delete(&request.into_inner().id, None) {
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

    async fn list(
        &self,
        _request: Request<ListTemplatesRequest>,
    ) -> Result<Response<ListTemplatesResponse>, Status> {
        let templates = match self.service.list() {
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
                branch: template.branch.clone(),
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
    use akira_core::{DeleteTemplateRequest, EventStream, ListTemplatesRequest, TemplateTrait};
    use akira_memory_stream::MemoryEventStream;
    use std::sync::Arc;
    use tonic::Request;

    use super::{GrpcTemplateService, TemplateMessage};

    use crate::models::Template;
    use crate::persistence::memory::MemoryPersistence;
    use crate::services::TemplateService;

    #[tokio::test]
    async fn test_create_list_template() -> anyhow::Result<()> {
        let template_persistence = Box::new(MemoryPersistence::<Template>::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let template_service = Arc::new(TemplateService {
            persistence: template_persistence,
            event_stream,
        });

        let template_grpc_service = GrpcTemplateService::new(Arc::clone(&template_service));

        let request = Request::new(TemplateMessage {
            id: "external-service".to_owned(),
            repository: "http://github.com/timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        });

        let create_response = template_grpc_service
            .create(request)
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

        let request = Request::new(DeleteTemplateRequest {
            id: "external-service".to_owned(),
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
