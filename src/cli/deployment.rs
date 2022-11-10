use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, ArgAction, Command};
use fabriq_core::{
    deployment::deployment_client::DeploymentClient, DeploymentIdRequest, DeploymentMessage,
    ListDeploymentsRequest, WorkloadMessage,
};
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command {
    Command::new("deployment")
        .arg_required_else_help(true)
        .long_flag("deployment")
        .about("manage deployments")
        .subcommand(
            Command::new("create")
                .about("create deployment")
                .arg(
                    Arg::new("hosts")
                        .long("hosts")
                        .help("host count for deployment (or 'all' for all matching hosts)")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("target")
                        .long("target")
                        .help("target id for deployment")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("team")
                        .long("team")
                        .help("team name for deployment")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("template")
                        .long("template")
                        .help("template override for deployment")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("workload")
                        .long("workload")
                        .help("workload name for deployment")
                        .action(ArgAction::Set),
                )
                .arg(arg!(<NAME> "deployment name"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("delete deployment")
                .arg(arg!(<ID> "deployment id"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List deployments"))
}

pub async fn handlers(model_match: &clap::ArgMatches, context: &Context) -> anyhow::Result<()> {
    let endpoint: &'static str = Box::leak(Box::new(context.endpoint.clone()));
    let channel = Channel::from_static(endpoint).connect().await?;

    let token = context.make_token()?;

    let mut client = DeploymentClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let deployment_name = add_match
                .get_one::<String>("NAME")
                .expect("deployment name expected")
                .to_string();
            let workload_name = add_match
                .get_one::<String>("workload")
                .expect("workload name expected")
                .to_string();
            let target_id = add_match
                .get_one::<String>("target")
                .expect("target expected")
                .to_string();
            let team = add_match
                .get_one::<String>("team")
                .expect("team expected")
                .to_string();
            let template_id: Option<String> = add_match
                .get_one::<String>("template")
                .map(|s| s.to_string());
            let host_count = add_match
                .get_one::<String>("hosts")
                .expect("hosts expected");

            let host_count = match host_count.as_str() {
                "all" => i32::MAX,
                _ => host_count.parse::<i32>()?,
            };

            let workload_id = WorkloadMessage::make_id(&team, &workload_name);
            let id = DeploymentMessage::make_id(&workload_id, &deployment_name);

            let request = tonic::Request::new(DeploymentMessage {
                id,
                name: deployment_name.clone(),
                workload_id,
                target_id,
                host_count,
                template_id,
            });

            client.upsert(request).await?;

            tracing::info!("deployment '{deployment_name}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match
                .get_one::<String>("ID")
                .expect("deployment id expected");

            let request = tonic::Request::new(DeploymentIdRequest {
                deployment_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("deployment '{id}' deleted");

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListDeploymentsRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .deployments
                .into_iter()
                .map(|deployment| {
                    let host_count = if deployment.host_count == i32::MAX {
                        "all".to_string()
                    } else {
                        deployment.host_count.to_string()
                    };

                    vec![
                        deployment.id.to_string(),
                        deployment.name.to_string(),
                        deployment.workload_id.clone(),
                        deployment.target_id.clone(),
                        deployment
                            .template_id
                            .unwrap_or_else(|| "(inherited)".to_string()),
                        host_count,
                    ]
                })
                .collect();

            if table_data.is_empty() {
                tracing::info!("no deployments found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("NAME")
                .set_align(Align::Left);

            ascii_table
                .column(2)
                .set_header("WORKLOAD ID")
                .set_align(Align::Left);

            ascii_table
                .column(3)
                .set_header("TARGET ID")
                .set_align(Align::Left);

            ascii_table
                .column(4)
                .set_header("TEMPLATE ID")
                .set_align(Align::Left);

            ascii_table
                .column(5)
                .set_header("HOSTS")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
