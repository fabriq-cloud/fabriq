use akira_core::target::target_client::TargetClient;
use akira_core::{DeleteTargetRequest, ListTargetsRequest, TargetMessage};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("target")
        .long_flag("target")
        .about("Manage targets")
        .subcommand(
            Command::new("create")
                .about("Create target")
                .arg(
                    Arg::new("label")
                        .short('l')
                        .long("label")
                        .help("Label to match for target")
                        .takes_value(true)
                        .multiple_values(true),
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

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.token.parse()?;

    let mut client = TargetClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let id = add_match
                .value_of("ID")
                .expect("Target name expected")
                .to_string();
            let labels = add_match
                .values_of("label")
                .expect("At least one label expected");

            let labels = labels.map(|s| s.to_string()).collect();

            println!("target create '{id}' w/ labels: {:?}", labels);

            let request = tonic::Request::new(TargetMessage { id, labels });

            client.create(request).await?;

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("Target id expected");

            println!("target delete '{id}'");

            let request = tonic::Request::new(DeleteTargetRequest { id: id.to_string() });

            client.delete(request).await?;

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
                println!("No targets found");

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
