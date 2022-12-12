use git2::{
    Cred, Direction, Index, ObjectType, PushOptions, RemoteCallbacks, Repository, Signature,
};
use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tempfile::TempDir;

use super::{GitRepo, GitRepoFactory};

pub struct ClonedGitRepo {
    pub index: Index,
    pub repository: Repository,

    pub local_path: TempDir,
}

pub struct RemoteGitRepo {
    pub cloned_repo: Mutex<Option<ClonedGitRepo>>,

    pub branch: String,
    pub private_ssh_key: String,
    pub repo_url: String,
}

impl Debug for RemoteGitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoteGitRepo")
    }
}

impl RemoteGitRepo {
    pub fn new(repo_url: &str, branch: &str, private_ssh_key: &str) -> anyhow::Result<Self> {
        Ok(Self {
            cloned_repo: Mutex::new(None),
            branch: branch.to_string(),
            private_ssh_key: private_ssh_key.to_string(),
            repo_url: repo_url.to_string(),
        })
    }

    fn get_auth_callback(private_ssh_key: &str) -> RemoteCallbacks {
        let mut auth_callback = RemoteCallbacks::new();

        auth_callback.credentials(|_url, username_from_url, _allowed_types| {
            let username = match username_from_url {
                Some(username) => username,
                None => return Err(git2::Error::from_str("No username found in URL")),
            };

            Cred::ssh_key_from_memory(username, None, private_ssh_key, None)
        });

        auth_callback
    }
}

#[derive(Debug)]
pub struct RemoteGitRepoFactory {}

impl GitRepoFactory for RemoteGitRepoFactory {
    fn create(
        &self,
        repo_url: &str,
        branch: &str,
        private_ssh_key: &str,
    ) -> anyhow::Result<Box<dyn GitRepo>> {
        Ok(Box::new(RemoteGitRepo::new(
            repo_url,
            branch,
            private_ssh_key,
        )?))
    }
}

impl GitRepo for RemoteGitRepo {
    #[tracing::instrument]
    fn add_path(&self, repo_path: PathBuf) -> anyhow::Result<()> {
        let mut cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_mut().unwrap();
        let repo_path = Path::new(&repo_path);
        Ok(cloned_repo.index.add_path(repo_path)?)
    }

    fn clone(&mut self) -> anyhow::Result<()> {
        let local_path = tempfile::tempdir()?;
        let auth_callback = RemoteGitRepo::get_auth_callback(&self.private_ssh_key);

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(auth_callback);

        // Prepare builder.
        let mut repo_builder = git2::build::RepoBuilder::new();
        repo_builder.fetch_options(fetch_options);

        let repository = repo_builder
            .branch(&self.branch)
            .clone(&self.repo_url, local_path.path())?;

        self.cloned_repo = Mutex::new(Some(ClonedGitRepo {
            index: repository.index()?,
            repository,
            local_path,
        }));

        Ok(())
    }

    #[tracing::instrument]
    fn commit(&self, name: &str, email: &str, message: &str) -> anyhow::Result<()> {
        let mut cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_mut().unwrap();

        let oid = cloned_repo.index.write_tree()?;

        let signature = Signature::now(name, email)?;

        let obj = cloned_repo
            .repository
            .head()?
            .resolve()?
            .peel(ObjectType::Commit)?;

        let parent_commit = obj
            .into_commit()
            .map_err(|err| anyhow::anyhow!("error: {:?}", err))?;

        let tree = cloned_repo.repository.find_tree(oid)?;

        cloned_repo.repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &[&parent_commit],
        )?;

        tracing::info!("commit completed on branch {}", self.branch);

        Ok(())
    }

    #[tracing::instrument]
    fn push(&self) -> anyhow::Result<()> {
        let cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_ref().unwrap();
        let mut remote = cloned_repo.repository.find_remote("origin")?;

        let connect_auth_callback = RemoteGitRepo::get_auth_callback(&self.private_ssh_key);
        remote.connect_auth(Direction::Push, Some(connect_auth_callback), None)?;

        let ref_spec = format!("refs/heads/{}:refs/heads/{}", self.branch, self.branch);

        let push_auth_callback = RemoteGitRepo::get_auth_callback(&self.private_ssh_key);
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(push_auth_callback);

        remote.push(&[ref_spec], Some(&mut push_options))?;

        tracing::info!("push completed on branch {}", self.branch);

        Ok(())
    }

    #[tracing::instrument]
    fn remove_dir(&self, path: &str) -> anyhow::Result<()> {
        let mut cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_mut().unwrap();
        Ok(cloned_repo.index.remove_dir(Path::new(&path), 0)?)
    }

    #[tracing::instrument]
    fn remove_file(&self, path: &str) -> anyhow::Result<()> {
        let mut cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_mut().unwrap();
        Ok(cloned_repo.index.remove_path(Path::new(path))?)
    }

    #[tracing::instrument]
    fn list(&self, repo_path: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
        let cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_ref().unwrap();
        let file_path = cloned_repo.local_path.path().join(repo_path);
        let directory = fs::read_dir(file_path)?;

        let mut entries = vec![];
        for entry in directory {
            entries.push(entry?.path());
        }

        Ok(entries)
    }

    #[tracing::instrument]
    fn read_file(&self, repo_path: PathBuf) -> anyhow::Result<Vec<u8>> {
        let cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_ref().unwrap();
        let file_path = cloned_repo.local_path.path().join(repo_path);
        let contents = fs::read(file_path)?;

        Ok(contents)
    }

    #[tracing::instrument]
    fn write_file(&self, repo_path: &str, contents: &[u8]) -> anyhow::Result<()> {
        let cloned_repo_option = self.cloned_repo.lock().unwrap();
        let cloned_repo = cloned_repo_option.as_ref().unwrap();

        let file_path = cloned_repo.local_path.path().join(repo_path);
        let directory_path = file_path.parent().unwrap();

        fs::create_dir_all(directory_path)?;

        tracing::info!(
            "writing file on branch {} path {}",
            self.branch,
            file_path.to_string_lossy()
        );
        fs::write(file_path, contents).expect("Unable to write host file");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        env,
        hash::{Hash, Hasher},
    };
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_clone_repo() {
        dotenvy::from_filename(".env.test").ok();

        let branch = "main";
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");
        let repo_url = "git@github.com:timfpark/akira-clone-repo-test.git";

        let mut gitops_repo = RemoteGitRepo::new(repo_url, branch, &private_ssh_key).unwrap();

        gitops_repo.clone().unwrap();

        let host_file = gitops_repo
            .read_file("hosts/azure-eastus2-1.yaml".into())
            .unwrap();

        let mut hasher = DefaultHasher::new();
        host_file.hash(&mut hasher);
        let deployment_hash = hasher.finish();

        assert_eq!(deployment_hash, 15629335063971853002);

        let data = Uuid::new_v4().to_string();

        gitops_repo
            .write_file("hosts/azure-eastus2-1.yaml", data.as_bytes())
            .unwrap();

        let contents_read = gitops_repo
            .read_file("hosts/azure-eastus2-1.yaml".into())
            .unwrap();

        assert_eq!(data, String::from_utf8(contents_read).unwrap());

        gitops_repo
            .commit(
                "Tim Park",
                "timfpark@gmail.com",
                "Create azure-eastus2-1 host",
            )
            .unwrap();

        gitops_repo.push().unwrap();
    }
}
