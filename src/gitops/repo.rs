use std::path::Path;

use git2::{
    Cred, Direction, Index, ObjectType, Oid, PushOptions, RemoteCallbacks, Repository, Signature,
};
use tokio::sync::Mutex;

pub struct GitRepo {
    pub index: Mutex<Index>,
    pub repository: Repository,

    pub branch: String,
    pub private_ssh_key: String,
}

impl GitRepo {
    pub fn new(
        local_path: &str,
        repo_url: &str,
        branch: &str,
        private_ssh_key: &str,
    ) -> anyhow::Result<Self> {
        let local_path = Path::new(local_path);

        let auth_callback = GitRepo::get_auth_callback(private_ssh_key);

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(auth_callback);

        // Prepare builder.
        let mut repo_builder = git2::build::RepoBuilder::new();
        repo_builder.fetch_options(fetch_options);

        let repository = repo_builder.clone(repo_url, local_path)?;

        let index = Mutex::new(repository.index()?);

        Ok(Self {
            branch: branch.to_string(),
            index,
            private_ssh_key: private_ssh_key.to_string(),
            repository,
        })
    }

    pub fn get_auth_callback(private_ssh_key: &str) -> RemoteCallbacks {
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

    pub async fn add(&self, path: &str) -> anyhow::Result<()> {
        let mut index = self.index.lock().await;
        Ok(index.add_path(Path::new(path))?)
    }

    pub async fn remove_dir(&self, path: &str) -> anyhow::Result<()> {
        let mut index = self.index.lock().await;
        Ok(index.remove_dir(Path::new(path), 0)?)
    }

    pub async fn commit(&self, name: &str, email: &str, message: &str) -> anyhow::Result<Oid> {
        let mut index = self.index.lock().await;
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

        Ok(self.repository.commit(
            Some("HEAD"), //  point HEAD to our new commit
            &signature,   // author
            &signature,   // committer
            message,      // commit message
            &tree,        // tree
            &[&parent_commit],
        )?)
    }

    pub fn push(&self) -> anyhow::Result<()> {
        let mut remote = self.repository.find_remote("origin")?;

        let connect_auth_callback = GitRepo::get_auth_callback(&self.private_ssh_key);
        remote.connect_auth(Direction::Push, Some(connect_auth_callback), None)?;

        let ref_spec = format!("refs/heads/{}:refs/heads/{}", self.branch, self.branch);

        let push_auth_callback = GitRepo::get_auth_callback(&self.private_ssh_key);
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(push_auth_callback);

        remote.push(&[ref_spec], Some(&mut push_options))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use dotenv::dotenv;
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_clone_repo() {
        dotenv().ok();

        let branch = "main";
        let local_path = "fixtures/temp";
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");
        let repo_url = "git@github.com:timfpark/akira-clone-repo-test.git";

        let _ = fs::remove_dir_all(local_path);
        fs::create_dir_all(local_path).unwrap();

        let gitops_repo = GitRepo::new(local_path, repo_url, branch, &private_ssh_key).unwrap();

        let hosts_path = format!("{}/hosts", local_path);
        let hosts_path = Path::new(&hosts_path);
        assert!(hosts_path.exists());

        let host_repo_path = "hosts/azure-eastus2-1.yaml";
        let host_path = format!("{}/{}", local_path, host_repo_path);
        let data = Uuid::new_v4().to_string();
        fs::write(host_path, data).expect("Unable to write host file");

        gitops_repo.add(host_repo_path).await.unwrap();

        gitops_repo
            .commit(
                "Tim Park",
                "timfpark@gmail.com",
                "Create azure-eastus2-1 host",
            )
            .await
            .unwrap();

        gitops_repo.push().unwrap();

        fs::remove_dir_all(local_path).unwrap();
    }
}
