use clap::Command;

mod assignment;
mod context;
mod deployment;
mod host;
mod target;
mod template;
mod workload;
mod workspace;

use context::Context;

fn cli() -> Command<'static> {
    Command::new("tatami")
        .about("declarative deployments")
        .version("0.1.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Tim Park")
        .subcommand(assignment::args())
        .subcommand(deployment::args())
        .subcommand(host::args())
        .subcommand(target::args())
        .subcommand(template::args())
        .subcommand(workload::args())
        .subcommand(workspace::args())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = cli().get_matches();

    let context = Context::default();

    match matches.subcommand() {
        Some(("assignment", submatches)) => Ok(assignment::handlers(submatches, &context).await?),
        Some(("deployment", submatches)) => Ok(deployment::handlers(submatches, &context).await?),
        Some(("host", submatches)) => Ok(host::handlers(submatches, &context).await?),
        Some(("target", submatches)) => Ok(target::handlers(submatches, &context).await?),
        Some(("template", submatches)) => Ok(template::handlers(submatches, &context).await?),
        Some(("workload", submatches)) => Ok(workload::handlers(submatches, &context).await?),
        Some(("workspace", submatches)) => Ok(workspace::handlers(submatches, &context).await?),
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
