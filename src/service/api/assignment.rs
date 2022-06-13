use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::models::Assignment;
use crate::services::AssignmentService;

use akira_core::{
    common::AssignmentIdRequest, AssignmentMessage, AssignmentTrait, DeploymentIdRequest,
    ListAssignmentsRequest, ListAssignmentsResponse, OperationId,
};

pub struct GrpcAssignmentService {
    service: Arc<AssignmentService>,
}

impl GrpcAssignmentService {
    pub fn new(service: Arc<AssignmentService>) -> Self {
        GrpcAssignmentService { service }
    }
}

#[tonic::async_trait]
impl AssignmentTrait for GrpcAssignmentService {
    async fn create(
        &self,
        request: Request<AssignmentMessage>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: Validate assignment id is valid

        let new_assignment = Assignment {
            id: request.get_ref().id.clone(),
            host_id: request.get_ref().host_id.clone(),
            deployment_id: request.get_ref().deployment_id.clone(),
        };

        let operation_id = match self.service.create(new_assignment, &None) {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::AlreadyExists,
                    format!("assignment {} already exists", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn delete(
        &self,
        request: Request<AssignmentIdRequest>,
    ) -> Result<Response<OperationId>, Status> {
        // TODO: check that no workloads are currently still using assignment
        // Query workload service for workloads by assignment_id, error if any exist

        let operation_id = match self
            .service
            .delete(&request.into_inner().assignment_id, &None)
        {
            Ok(operation_id) => operation_id,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!("assignment with id {} not found", err),
                ))
            }
        };

        Ok(Response::new(operation_id))
    }

    async fn get_by_deployment_id(
        &self,
        request: Request<DeploymentIdRequest>,
    ) -> Result<Response<ListAssignmentsResponse>, Status> {
        let deployment_id = request.into_inner().deployment_id;
        let assignments = match self.service.get_by_deployment_id(&deployment_id) {
            Ok(assignments) => assignments,
            Err(err) => {
                tracing::error!(
                    "get assignments with deployment id {} failed: {}",
                    deployment_id,
                    err
                );
                return Err(Status::new(
                    tonic::Code::NotFound,
                    format!(
                        "get assignments with deployment id {} failed",
                        deployment_id
                    ),
                ));
            }
        };

        let assignments: Vec<AssignmentMessage> = assignments
            .into_iter()
            .map(|assignment| assignment.into())
            .collect();
        let response = ListAssignmentsResponse { assignments };
        Ok(Response::new(response))
    }

    async fn list(
        &self,
        _request: Request<ListAssignmentsRequest>,
    ) -> Result<Response<ListAssignmentsResponse>, Status> {
        let assignments = match self.service.list() {
            Ok(assignments) => assignments,
            Err(err) => {
                return Err(Status::new(
                    tonic::Code::Internal,
                    format!("listing assignments failed with {}", err),
                ))
            }
        };

        let assignment_messages = assignments
            .iter()
            .map(|assignment| AssignmentMessage {
                id: assignment.id.clone(),
                host_id: assignment.host_id.clone(),
                deployment_id: assignment.deployment_id.clone(),
            })
            .collect();

        let response = ListAssignmentsResponse {
            assignments: assignment_messages,
        };

        Ok(Response::new(response))
    }
}

#[cfg(test)]
mod tests {
    use akira_core::common::AssignmentIdRequest;
    use std::sync::Arc;
    use tonic::Request;

    use akira_core::{AssignmentMessage, AssignmentTrait, EventStream, ListAssignmentsRequest};
    use akira_memory_stream::MemoryEventStream;

    use crate::api::GrpcAssignmentService;
    use crate::persistence::memory::AssignmentMemoryPersistence;
    use crate::services::AssignmentService;

    #[tokio::test]
    async fn test_create_list_assignment() -> anyhow::Result<()> {
        let assignment_persistence = Box::new(AssignmentMemoryPersistence::default());
        let event_stream =
            Arc::new(Box::new(MemoryEventStream::new().unwrap()) as Box<dyn EventStream + 'static>);

        let assignment_service = Arc::new(AssignmentService {
            persistence: assignment_persistence,
            event_stream,
        });

        let assignment_grpc_service = GrpcAssignmentService::new(Arc::clone(&assignment_service));

        let request = Request::new(AssignmentMessage {
            id: "assignment-grpc-test".to_string(),
            host_id: "host-fixture".to_string(),
            deployment_id: "deployment-fixture".to_string(),
        });

        let response = assignment_grpc_service
            .create(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        let request = Request::new(ListAssignmentsRequest {});
        let _ = assignment_grpc_service
            .list(request)
            .await
            .unwrap()
            .into_inner();

        let request = Request::new(AssignmentIdRequest {
            assignment_id: "assignment-grpc-test".to_string(),
        });
        let response = assignment_grpc_service
            .delete(request)
            .await
            .unwrap()
            .into_inner();

        assert_eq!(response.id.len(), 36);

        Ok(())
    }
}
