use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

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

pub struct MemoryGitRepo {
    cloned_repo: Arc<MemoryClonedGitRepo>,
}

impl MemoryGitRepo {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            cloned_repo: Arc::new(MemoryClonedGitRepo::new()),
        })
    }
}

impl GitRepo for MemoryGitRepo {
    fn clone_repo(&self) -> anyhow::Result<Arc<dyn ClonedGitRepo>> {
        let coerced_repo = Arc::clone(&self.cloned_repo) as Arc<dyn ClonedGitRepo>;

        Ok(coerced_repo)
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

        let matching_keys: Vec<String> = files
            .iter()
            .filter(|(key, _)| key.starts_with(path))
            .map(|(key, _)| key.clone())
            .collect();

        matching_keys.iter().for_each(|key| {
            files.remove(key);
        });

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
        let repo = MemoryGitRepo::new()?;

        let cloned_repo = repo.clone_repo()?;

        const CONTENTS: &[u8] = b"Hello, world!";
        const PATH: &str =
            "deployments/workspace-fixture/workload-fixture/deployment-fixture/deployment.yaml";

        cloned_repo.write_file(PATH, CONTENTS)?;

        let cloned_repo = repo.clone_repo()?;

        let read_contents = cloned_repo.read_file(PATH.into())?;
        assert_eq!(read_contents, CONTENTS);

        let list = cloned_repo.list(PATH.into()).unwrap();
        assert_eq!(list.len(), 1);

        Ok(())
    }
}
