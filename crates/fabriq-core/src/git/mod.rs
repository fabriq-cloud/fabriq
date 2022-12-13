pub mod memory;
pub mod remote;
mod repo;

pub use memory::MemoryGitRepo;
pub use remote::RemoteGitRepo;
pub use repo::{ClonedGitRepo, GitRepo, GitRepoFactory};
