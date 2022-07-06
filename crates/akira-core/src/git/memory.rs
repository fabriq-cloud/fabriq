use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use super::GitRepo;

#[derive(Debug)]
pub struct MemoryGitRepo {
    pub files: Mutex<HashMap<String, Vec<u8>>>, // path -> contents
}

impl MemoryGitRepo {
    pub fn new() -> Self {
        MemoryGitRepo {
            files: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryGitRepo {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryGitRepo {
    pub fn read_file(&self, repo_path: PathBuf) -> anyhow::Result<Vec<u8>> {
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
}

impl GitRepo for MemoryGitRepo {
    fn add_path(&self, _repo_path: PathBuf) -> anyhow::Result<()> {
        Ok(())
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

    fn commit(&self, _name: &str, _email: &str, _message: &str) -> anyhow::Result<()> {
        Ok(())
    }

    fn push(&self) -> anyhow::Result<()> {
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
    use crate::git::GitRepo;

    #[test]
    fn test_write_read_file() -> anyhow::Result<()> {
        let repo = super::MemoryGitRepo::new();

        let contents = b"Hello, world!";
        let path =
            "deployments/workspace-fixture/workload-fixture/deployment-fixture/deployment.yaml";

        repo.write_file(path, contents)?;
        let read_contents = repo.read_file(path.into())?;
        assert_eq!(read_contents, contents);

        Ok(())
    }
}
