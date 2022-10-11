use clap::{arg, Command};

use crate::{context::Context, profile::Profile};

pub fn args() -> Command<'static> {
    Command::new("login")
        .long_flag("login")
        .arg(arg!(<PAT> "GitHub Personal Access Token"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    _context: &Context<'static>,
) -> anyhow::Result<()> {
    let pat = model_match
        .value_of("PAT")
        .expect("GitHub Personal Access Token expected")
        .to_string();

    let octocrab = octocrab::OctocrabBuilder::new()
        .personal_token(pat.clone())
        .build()?;

    let github_user = octocrab.current().user().await?;

    let profile = Profile {
        pat: Some(pat),
        login: Some(github_user.login),
    };

    profile.save()?;

    tracing::info!(
        "logged in - saving Github user context as {:?}",
        profile.login.unwrap()
    );

    Ok(())
}
