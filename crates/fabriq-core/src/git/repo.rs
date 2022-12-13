use std::fmt::Debug;
use std::path::PathBuf;

pub trait GitRepo {
    fn clone_repo(&self) -> anyhow::Result<Box<dyn ClonedGitRepo>>;
}

pub trait ClonedGitRepo {
    fn add_path(&self, repo_path: PathBuf) -> anyhow::Result<()>;
    fn commit(&self, name: &str, email: &str, message: &str) -> anyhow::Result<()>;
    fn list(&self, repo_path: PathBuf) -> anyhow::Result<Vec<PathBuf>>;
    fn push(&self) -> anyhow::Result<()>;
    fn read_file(&self, repo_path: PathBuf) -> anyhow::Result<Vec<u8>>;
    fn remove_dir(&self, path: &str) -> anyhow::Result<()>;
    fn remove_file(&self, path: &str) -> anyhow::Result<()>;
    fn write_file(&self, repo_path: &str, contents: &[u8]) -> anyhow::Result<()>;
}

pub trait GitRepoFactory: Debug {
    fn create(
        &self,
        repository_url: &str,
        branch: &str,
        private_ssh_key: &str,
    ) -> anyhow::Result<Box<dyn GitRepo>>;
}
