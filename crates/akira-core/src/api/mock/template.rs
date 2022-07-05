use tonic::{Request, Response, Status};

use crate::{
    common::TemplateIdRequest, ListTemplatesRequest, ListTemplatesResponse, OperationId,
    TemplateMessage, TemplateTrait,
};

pub struct MockTemplateClient {}

#[tonic::async_trait]
impl TemplateTrait for MockTemplateClient {
    async fn create(
        &self,
        _request: Request<TemplateMessage>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn delete(
        &self,
        _request: Request<TemplateIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        Ok(Response::new(OperationId::create()))
    }

    async fn get_by_id(
        &self,
        _request: Request<TemplateIdRequest>,
    ) -> Result<Response<TemplateMessage>, Status> {
        Ok(Response::new(TemplateMessage {
            id: "template-fixture".to_owned(),
            repository: "git@github.com:timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        }))
    }

    async fn list(
        &self,
        _request: Request<ListTemplatesRequest>,
    ) -> Result<Response<ListTemplatesResponse>, Status> {
        let template = TemplateMessage {
            id: "external-service".to_owned(),
            repository: "http://github.com/timfpark/deployment-templates".to_owned(),
            branch: "main".to_owned(),
            path: "external-service".to_owned(),
        };

        Ok(Response::new(ListTemplatesResponse {
            templates: vec![template],
        }))
    }
}
