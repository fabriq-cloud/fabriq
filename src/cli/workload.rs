use akira_core::workload::workload_client::WorkloadClient;
use akira_core::{DeleteWorkloadRequest, ListWorkloadsRequest, WorkloadMessage};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("workload")
        .long_flag("workload")
        .about("Manage workloads")
        .subcommand(
            Command::new("create")
                .about("Create workload")
                .arg(
                    Arg::new("workspace")
                        .short('w')
                        .long("workspace")
                        .help("Workspace this workload belongs to")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("template")
                        .short('t')
                        .long("template")
                        .help("Template this workload should use")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(arg!(<ID> "Workload ID"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete workload")
                .arg(arg!(<ID> "ID of workload"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List workloads"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    // TODO: Can this be made generic?
    let channel = Channel::from_static(&context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.token.parse()?;

    let mut client = WorkloadClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let id = add_match
                .value_of("ID")
                .expect("Workload name expected")
                .to_string();
            let workspace_id = add_match
                .value_of("workspace")
                .expect("Workspace ID expected")
                .to_string();
            let template_id = add_match
                .value_of("template")
                .expect("Template ID expected")
                .to_string();

            println!("workload create '{id}'");

            let request = tonic::Request::new(WorkloadMessage {
                id,
                workspace_id,
                template_id,
            });

            client.create(request).await?;

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("Workload id expected");

            println!("workload delete '{id}'");

            let request = tonic::Request::new(DeleteWorkloadRequest { id: id.to_string() });

            client.delete(request).await?;

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListWorkloadsRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .workloads
                .into_iter()
                .map(|workload| {
                    vec![
                        workload.id.to_string(),
                        workload.workspace_id.clone(),
                        workload.template_id.clone(),
                    ]
                })
                .collect();

            if table_data.len() == 0 {
                println!("No workloads found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("WORKSPACE ID")
                .set_align(Align::Left);

            ascii_table
                .column(2)
                .set_header("TEMPLATE ID")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
