mod assignment;
mod config;
mod deployment;
mod health;
mod host;
mod target;
mod template;
mod workload;
mod workspace;

pub use assignment::GrpcAssignmentService;
pub use config::GrpcConfigService;
pub use deployment::GrpcDeploymentService;
pub use health::GrpcHealthService;
pub use host::GrpcHostService;
pub use target::GrpcTargetService;
pub use template::GrpcTemplateService;
pub use workload::GrpcWorkloadService;
pub use workspace::GrpcWorkspaceService;
