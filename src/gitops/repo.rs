use std::path::Path;

use git2::{build::RepoBuilder, Cred, RemoteCallbacks};

pub struct GitRepo {
    pub local_path: String,
    pub repo_url: String,
    pub private_ssh_key: String,
}

impl GitRepo {
    pub fn new(local_path: &str, repo_url: &str, private_ssh_key_path: &str) -> Self {
        Self {
            local_path: local_path.to_string(),
            repo_url: repo_url.to_string(),
            private_ssh_key: private_ssh_key_path.to_string(),
        }
    }

    fn get_auth_callback(&self) -> RemoteCallbacks {
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key_from_memory(
                username_from_url.unwrap(),
                None,
                &self.private_ssh_key,
                None,
            )
        });

        callbacks
    }

    fn create_repo_builder(&self) -> RepoBuilder {
        let auth_callback = self.get_auth_callback();

        // Prepare fetch options.
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(auth_callback);

        // Prepare builder.
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_options);

        builder
    }

    pub fn clone(&self) -> anyhow::Result<()> {
        let local_path = Path::new(&self.local_path);
        let mut repo_builder = self.create_repo_builder();

        match repo_builder.clone(&self.repo_url, local_path) {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow! {
                format!("Failed to clone repo: {}", err)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use dotenv::dotenv;

    use super::*;

    #[test]
    fn test_clone_repo() {
        dotenv().ok();

        let local_path = "fixtures/temp";
        let repo_url = "git@github.com:timfpark/akira-clone-repo-test.git";
        let private_ssh_key = env::var("PRIVATE_SSH_KEY").expect("PRIVATE_SSH_KEY must be set");

        let _ = fs::remove_dir_all(local_path);
        fs::create_dir_all(local_path).unwrap();

        let gitops_repo = GitRepo::new(local_path, repo_url, &private_ssh_key);

        gitops_repo.clone().unwrap();

        let hosts_path = format!("{}/hosts", local_path);
        let hosts_path = Path::new(&hosts_path);
        assert!(hosts_path.exists());

        fs::remove_dir_all(local_path).unwrap();
    }
}
