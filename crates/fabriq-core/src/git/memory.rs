use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use super::{ClonedGitRepo, GitRepo};

#[derive(Debug)]
pub struct MemoryClonedGitRepo {
    pub files: Mutex<HashMap<String, Vec<u8>>>, // path -> contents
}

impl MemoryClonedGitRepo {
    pub fn new() -> Self {
        MemoryClonedGitRepo {
            files: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryClonedGitRepo {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MemoryGitRepo {}

impl GitRepo for MemoryGitRepo {
    fn clone_repo(&self) -> anyhow::Result<Box<dyn ClonedGitRepo>> {
        Ok(Box::new(MemoryClonedGitRepo::default()))
    }
}

impl ClonedGitRepo for MemoryClonedGitRepo {
    fn add_path(&self, _repo_path: PathBuf) -> anyhow::Result<()> {
        Ok(())
    }

    fn commit(&self, _name: &str, _email: &str, _message: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn push(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn list(&self, repo_path: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        let mut path_files = vec![];

        for (path, _) in files.iter() {
            if path.starts_with(repo_path.to_str().unwrap()) {
                path_files.push(PathBuf::from(path));
            }
        }

        Ok(path_files)
    }

    fn read_file(&self, repo_path: PathBuf) -> anyhow::Result<Vec<u8>> {
        let files = self.files.lock().unwrap();
        let file_contents = files.get(&repo_path.to_string_lossy().to_string());
        match file_contents {
            Some(file_contents) => Ok(file_contents.clone()),
            None => Err(anyhow::anyhow!(
                "File not found: {}",
                repo_path.to_string_lossy()
            )),
        }
    }

    fn remove_dir(&self, path: &str) -> anyhow::Result<()> {
        let mut files = self.files.lock().unwrap();

        *files = files
            .iter()
            .filter(|(key, _)| key.starts_with(path))
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect();

        Ok(())
    }

    fn remove_file(&self, path: &str) -> anyhow::Result<()> {
        let mut files = self.files.lock().unwrap();

        files.remove(path);

        Ok(())
    }

    fn write_file(&self, repo_path: &str, contents: &[u8]) -> anyhow::Result<()> {
        let mut files = self.files.lock().unwrap();

        files.insert(repo_path.to_string(), contents.to_vec());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitRepo;

    #[test]
    fn test_write_read_file() -> anyhow::Result<()> {
        let repo = MemoryGitRepo {};

        let cloned_repo = repo.clone_repo()?;

        let contents = b"Hello, world!";
        let path =
            "deployments/workspace-fixture/workload-fixture/deployment-fixture/deployment.yaml";

        cloned_repo.write_file(path, contents)?;
        let read_contents = cloned_repo.read_file(path.into())?;
        assert_eq!(read_contents, contents);

        let list = cloned_repo.list(path.into()).unwrap();
        assert_eq!(list.len(), 1);

        Ok(())
    }
}
