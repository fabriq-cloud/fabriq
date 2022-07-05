mod deployment;
mod interceptor;
mod template;
mod workload;

pub use deployment::WrappedDeploymentClient;
pub use template::WrappedTemplateClient;
pub use workload::WrappedWorkloadClient;
