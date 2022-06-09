use akira_core::assignment::assignment_client::AssignmentClient;
use akira_core::{AssignmentMessage, DeleteAssignmentRequest, ListAssignmentsRequest};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("assignment")
        .long_flag("assignment")
        .about("manage assignments")
        .subcommand(
            Command::new("create")
                .about("create assignment")
                .arg(
                    Arg::new("deployment")
                        .long("deployment")
                        .help("deployment id for assignment")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("host")
                        .long("host")
                        .help("host id for assignment")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("delete assignment")
                .arg(arg!(<ID> "assignment id"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List assignments"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    // TODO: Can this be made generic?
    let channel = Channel::from_static(&context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.token.parse()?;

    let mut client = AssignmentClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let deployment_id = add_match
                .value_of("deployment")
                .expect("deployment id expected")
                .to_string();

            let host_id = add_match
                .value_of("host")
                .expect("host id expected")
                .to_string();

            let id = format!("{}|{}", deployment_id, host_id);

            println!("assignment create '{id}'");

            let request = tonic::Request::new(AssignmentMessage {
                id,
                deployment_id,
                host_id,
            });

            client.create(request).await?;

            Ok(())
        }
        Some(("delete", delete_match)) => {
            let id = delete_match.value_of("ID").expect("assignment id expected");

            println!("assignment delete '{id}'");

            let request = tonic::Request::new(DeleteAssignmentRequest { id: id.to_string() });

            client.delete(request).await?;

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListAssignmentsRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .assignments
                .into_iter()
                .map(|assignment| {
                    vec![
                        assignment.id.to_string(),
                        assignment.deployment_id.clone(),
                        assignment.host_id.clone(),
                    ]
                })
                .collect();

            if table_data.len() == 0 {
                println!("No assignments found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("DEPLOYMENT ID")
                .set_align(Align::Left);

            ascii_table
                .column(2)
                .set_header("HOST ID")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
