use octocrab::{models::User, Octocrab};
use tonic::Status;

pub async fn build_octocrab_client(pat: &str) -> Result<Octocrab, Status> {
    let octocrab = octocrab::OctocrabBuilder::new()
        .personal_token(pat.to_string())
        .build()
        .map_err(|_| Status::new(tonic::Code::Internal, "failed to create octocrab instance"))?;

    Ok(octocrab)
}

pub async fn get_user(octocrab: &Octocrab) -> Result<User, Status> {
    let user = match octocrab.get("/user", None::<&()>).await {
        Ok(user) => user,
        Err(_) => {
            return Err(Status::new(
                tonic::Code::Internal,
                "failed to get user for PAT from github",
            ))
        }
    };

    Ok(user)
}

pub async fn get_team_members(
    octocrab: &Octocrab,
    org: &str,
    team: &str,
) -> Result<Vec<User>, Status> {
    let url = format!("/orgs/{org}/teams/{team}/members");
    let team_members: Vec<User> = match octocrab.get(url, None::<&()>).await {
        Ok(team_members) => team_members,
        Err(_) => {
            return Err(Status::new(
                tonic::Code::Internal,
                "failed to get members for team from github",
            ))
        }
    };

    Ok(team_members)
}

pub async fn is_team_member(pat: &str, team_id: &str) -> Result<bool, Status> {
    let (org, team) = match team_id.split_once('/') {
        Some((org, team)) => (org, team),
        None => {
            return Err(Status::new(
                tonic::Code::Internal,
                "failed to create octocrab instance",
            ));
        }
    };

    let octocrab = build_octocrab_client(pat).await?;

    let user = get_user(&octocrab).await?;
    let team_members = match get_team_members(&octocrab, org, team).await {
        Ok(team_members) => team_members,
        Err(_) => return Ok(false),
    };

    for team_member in team_members {
        if team_member.login == user.login {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_user() -> anyhow::Result<()> {
        let pat = std::env::var("GITHUB_TOKEN")?;

        let octocrab = build_octocrab_client(&pat).await?;
        get_user(&octocrab).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_get_team_members() -> anyhow::Result<()> {
        let pat = std::env::var("GITHUB_TOKEN")?;

        let octocrab = build_octocrab_client(&pat).await?;
        let members = get_team_members(&octocrab, "fabriq-cloud", "fabriq").await?;

        assert!(!members.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_is_team_member() -> anyhow::Result<()> {
        let pat = std::env::var("GITHUB_TOKEN")?;

        let is_member = is_team_member(&pat, "fabriq-cloud/fabriq").await?;

        assert!(is_member);

        Ok(())
    }

    #[tokio::test]
    async fn test_is_not_team_member() -> anyhow::Result<()> {
        let pat = std::env::var("GITHUB_TOKEN")?;

        let is_member = is_team_member(&pat, "another-cloud/team").await?;

        assert!(!is_member);

        Ok(())
    }
}
