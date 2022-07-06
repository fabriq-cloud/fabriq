use std::fmt::Debug;
use std::path::PathBuf;

pub trait GitRepo: Debug {
    fn add_path(&self, repo_path: PathBuf) -> anyhow::Result<()>;
    fn remove_dir(&self, path: &str) -> anyhow::Result<()>;
    fn commit(&self, name: &str, email: &str, message: &str) -> anyhow::Result<()>;
    fn push(&self) -> anyhow::Result<()>;
    fn write_file(&self, repo_path: &str, contents: &[u8]) -> anyhow::Result<()>;
}
