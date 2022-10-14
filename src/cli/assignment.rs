use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, ArgAction, Command};
use fabriq_core::{
    assignment::assignment_client::AssignmentClient, common::AssignmentIdRequest,
    AssignmentMessage, ListAssignmentsRequest,
};
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command {
    Command::new("assignment")
        .arg_required_else_help(true)
        .about("manage assignments")
        .subcommand(
            Command::new("create")
                .about("create assignment")
                .arg(
                    Arg::new("deployment")
                        .long("deployment")
                        .help("deployment id for assignment")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("host")
                        .long("host")
                        .help("host id for assignment")
                        .action(ArgAction::Set),
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
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token = context.make_token()?;

    let mut client = AssignmentClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let deployment_id = add_match
                .get_one::<String>("deployment")
                .expect("deployment id expected")
                .to_string();

            let host_id = add_match
                .get_one::<String>("host")
                .expect("host id expected")
                .to_string();

            let id = format!("{}|{}", deployment_id, host_id);

            let request = tonic::Request::new(AssignmentMessage {
                id: id.clone(),
                deployment_id,
                host_id,
            });

            client.upsert(request).await?;

            tracing::info!("assignment '{id}' created");

            Ok(())
        }
        Some(("delete", delete_match)) => {
            let id = delete_match
                .get_one::<String>("ID")
                .expect("assignment id expected");

            let request = tonic::Request::new(AssignmentIdRequest {
                assignment_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("assignment '{id}' deleted");

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
                        assignment.host_id,
                    ]
                })
                .collect();

            if table_data.is_empty() {
                tracing::info!("no assignments found");

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
