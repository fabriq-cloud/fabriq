mod assignment;
mod deployment;
mod generic;
mod host;
mod workload;

pub use assignment::AssignmentMemoryPersistence;
pub use deployment::DeploymentMemoryPersistence;
pub use generic::MemoryPersistence;
pub use host::HostMemoryPersistence;
pub use workload::WorkloadMemoryPersistence;
