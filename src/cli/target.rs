use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, ArgAction, Command};
use fabriq_core::{
    target::target_client::TargetClient, ListTargetsRequest, TargetIdRequest, TargetMessage,
};
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command {
    Command::new("target")
        .arg_required_else_help(true)
        .about("Manage targets")
        .subcommand(
            Command::new("create")
                .about("Create target")
                .arg(
                    Arg::new("label")
                        .short('l')
                        .long("label")
                        .help("Label to match for target")
                        .action(ArgAction::Set)
                        .num_args(1..),
                )
                .arg(arg!(<ID> "Target ID"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete target")
                .arg(arg!(<ID> "ID of target"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List targets"))
}

pub async fn handlers(model_match: &clap::ArgMatches, context: &Context) -> anyhow::Result<()> {
    let endpoint: &'static str = Box::leak(Box::new(context.endpoint.clone()));
    let channel = Channel::from_static(endpoint).connect().await?;

    let token = context.make_token()?;

    let mut client = TargetClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let id = add_match
                .get_one::<String>("ID")
                .expect("Target name expected")
                .to_string();
            let labels = add_match
                .get_many::<String>("label")
                .expect("At least one label expected");

            let labels = labels.map(|s| s.to_string()).collect();

            let request = tonic::Request::new(TargetMessage {
                id: id.clone(),
                labels,
            });

            client.upsert(request).await?;

            tracing::info!("target '{id}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match
                .get_one::<String>("ID")
                .expect("Target id expected");

            let request = tonic::Request::new(TargetIdRequest {
                target_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("target '{id}' deleted");

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListTargetsRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .targets
                .into_iter()
                .map(|target| vec![target.id.to_string(), target.labels.join(", ")])
                .collect();

            if table_data.is_empty() {
                tracing::info!("no targets found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("LABELS")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
