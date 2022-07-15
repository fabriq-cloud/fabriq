use akira_core::deployment::deployment_client::DeploymentClient;
use akira_core::{DeploymentIdRequest, DeploymentMessage, ListDeploymentsRequest};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("deployment")
        .long_flag("deployment")
        .about("manage deployments")
        .subcommand(
            Command::new("create")
                .about("create deployment")
                .arg(
                    Arg::new("workload")
                        .long("workload")
                        .help("workload id for deployment")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("target")
                        .long("target")
                        .help("target id for deployment")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("template")
                        .long("template")
                        .help("template override for deployment")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("hosts")
                        .long("hosts")
                        .help("host count for deployment")
                        .takes_value(true)
                        .multiple_values(false),
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

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token: MetadataValue<Ascii> = context.profile.pat.parse()?;

    let mut client = DeploymentClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let deployment_name = add_match
                .value_of("NAME")
                .expect("Deployment name expected")
                .to_string();
            let workload_id = add_match
                .value_of("workload")
                .expect("Workload ID expected")
                .to_string();
            let target_id = add_match
                .value_of("target")
                .expect("Target ID expected")
                .to_string();
            let template_id: Option<String> = add_match.value_of("template").map(|s| s.to_string());
            let host_count = add_match
                .value_of("hosts")
                .expect("hosts expected")
                .parse::<i32>()?;

            let request = tonic::Request::new(DeploymentMessage {
                id: DeploymentMessage::make_id(&workload_id, &deployment_name),
                name: deployment_name.clone(),
                workload_id,
                target_id,
                host_count,
                template_id,
            });

            client.create(request).await?;

            tracing::info!("deployment '{deployment_name}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("deployment id expected");

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
                    vec![
                        deployment.id.to_string(),
                        deployment.name.to_string(),
                        deployment.workload_id.clone(),
                        deployment.target_id.clone(),
                        deployment
                            .template_id
                            .unwrap_or_else(|| "(inherited)".to_string()),
                        deployment.host_count.to_string(),
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
                .set_header("HOST COUNT")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
