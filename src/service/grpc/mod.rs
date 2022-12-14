mod assignment;
mod auth;
mod config;
mod deployment;
mod host;
mod target;
mod template;
mod workload;

pub use assignment::GrpcAssignmentService;
pub use config::GrpcConfigService;
pub use deployment::GrpcDeploymentService;
pub use host::GrpcHostService;
pub use target::GrpcTargetService;
pub use template::GrpcTemplateService;
pub use workload::GrpcWorkloadService;
