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

pub struct RemoteGitRepo {
    pub index: Mutex<Index>,
    pub repository: Repository,

    pub branch: String,
    pub private_ssh_key: String,
    pub local_path: TempDir,
}

impl Debug for RemoteGitRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RemoteGitRepo")
    }
}

impl RemoteGitRepo {
    pub fn new(repo_url: &str, branch: &str, private_ssh_key: &str) -> anyhow::Result<Self> {
        let local_path = tempfile::tempdir()?;
        let auth_callback = RemoteGitRepo::get_auth_callback(private_ssh_key);

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(auth_callback);

        // Prepare builder.
        let mut repo_builder = git2::build::RepoBuilder::new();
        repo_builder.fetch_options(fetch_options);

        let repository = repo_builder
            .branch(branch)
            .clone(repo_url, local_path.path())?;

        let index = Mutex::new(repository.index()?);

        Ok(Self {
            branch: branch.to_string(),
            index,
            private_ssh_key: private_ssh_key.to_string(),
            repository,
            local_path,
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
        let mut index = self.index.lock().unwrap();
        let repo_path = Path::new(&repo_path);
        Ok(index.add_path(repo_path)?)
    }

    #[tracing::instrument]
    fn commit(&self, name: &str, email: &str, message: &str) -> anyhow::Result<()> {
        let mut index = self.index.lock().unwrap();

        let oid = index.write_tree()?;

        let signature = Signature::now(name, email)?;

        let obj = self
            .repository
            .head()?
            .resolve()?
            .peel(ObjectType::Commit)?;

        let parent_commit = obj
            .into_commit()
            .map_err(|err| anyhow::anyhow!("error: {:?}", err))?;

        let tree = self.repository.find_tree(oid)?;

        self.repository.commit(
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
        let mut remote = self.repository.find_remote("origin")?;

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
        let mut index = self.index.lock().unwrap();
        Ok(index.remove_dir(Path::new(&path), 0)?)
    }

    #[tracing::instrument]
    fn remove_file(&self, path: &str) -> anyhow::Result<()> {
        let mut index = self.index.lock().unwrap();
        Ok(index.remove_path(Path::new(path))?)
    }

    #[tracing::instrument]
    fn list(&self, repo_path: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
        let file_path = self.local_path.path().join(repo_path);
        let directory = fs::read_dir(file_path)?;

        let mut entries = vec![];
        for entry in directory {
            entries.push(entry?.path());
        }

        Ok(entries)
    }

    #[tracing::instrument]
    fn read_file(&self, repo_path: PathBuf) -> anyhow::Result<Vec<u8>> {
        let file_path = self.local_path.path().join(repo_path);
        let contents = fs::read(file_path)?;

        Ok(contents)
    }

    #[tracing::instrument]
    fn write_file(&self, repo_path: &str, contents: &[u8]) -> anyhow::Result<()> {
        let file_path = self.local_path.path().join(repo_path);
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
        dotenv::from_filename(".env.test").ok();

        let branch = "main";
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");
        let repo_url = "git@github.com:timfpark/akira-clone-repo-test.git";

        let gitops_repo = RemoteGitRepo::new(repo_url, branch, &private_ssh_key).unwrap();

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
