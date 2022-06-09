use akira_core::workspace::workspace_client::WorkspaceClient;
use akira_core::{DeleteWorkspaceRequest, WorkspaceMessage, ListWorkspacesRequest};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("workspace")
        .long_flag("workspace")
        .about("Manage workspaces")
        .subcommand(
            Command::new("create")
                .about("Create workspace")
                .arg(arg!(<ID> "Name of workspace"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete workspace")
                .arg(arg!(<ID> "ID of workspace"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List workspaces"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.token.parse()?;

    let mut client = WorkspaceClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", create_match)) => {
            let id = create_match.value_of("ID").expect("workspace id expected");

            // validate that id has no spaces

            println!("workspace create '{id}'");

            let request = tonic::Request::new(WorkspaceMessage { id: id.to_string() });

            client.create(request).await?;

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("workspace id expected");

            println!("workspace delete '{id}'");

            let request = tonic::Request::new(DeleteWorkspaceRequest { id: id.to_string() });

            client.delete(request).await?;

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListWorkspacesRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .workspaces
                .into_iter()
                .map(|workspace| vec![workspace.id])
                .collect();

            if table_data.is_empty() {
                println!("no workspaces found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
