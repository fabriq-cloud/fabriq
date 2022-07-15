use clap::Command;

mod assignment;
mod config;
mod context;
mod deployment;
mod host;
mod login;
mod profile;
mod target;
mod team;
mod template;
mod workload;

use context::Context;

fn cli() -> Command<'static> {
    Command::new("akira")
        .about("scaled declarative deployments")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Tim Park")
        .subcommand(assignment::args())
        .subcommand(config::args())
        .subcommand(deployment::args())
        .subcommand(host::args())
        .subcommand(login::args())
        .subcommand(target::args())
        .subcommand(template::args())
        .subcommand(workload::args())
        .subcommand(team::args())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let matches = cli().get_matches();

    let context = Context::default();

    match matches.subcommand() {
        Some(("assignment", submatches)) => Ok(assignment::handlers(submatches, &context).await?),
        Some(("config", submatches)) => Ok(config::handlers(submatches, &context).await?),
        Some(("deployment", submatches)) => Ok(deployment::handlers(submatches, &context).await?),
        Some(("host", submatches)) => Ok(host::handlers(submatches, &context).await?),
        Some(("login", submatches)) => Ok(login::handlers(submatches, &context).await?),
        Some(("target", submatches)) => Ok(target::handlers(submatches, &context).await?),
        Some(("team", submatches)) => Ok(team::handlers(submatches, &context).await?),
        Some(("template", submatches)) => Ok(template::handlers(submatches, &context).await?),
        Some(("workload", submatches)) => Ok(workload::handlers(submatches, &context).await?),
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
