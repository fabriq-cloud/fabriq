use akira_core::host::host_client::HostClient;
use akira_core::{DeleteHostRequest, HostMessage, ListHostsRequest};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("host")
        .long_flag("host")
        .about("manage hosts")
        .subcommand(
            Command::new("create")
                .about("create host")
                .arg(
                    Arg::new("label")
                        .short('l')
                        .long("label")
                        .help("label to apply to host")
                        .takes_value(true)
                        .multiple_values(true),
                )
                .arg(arg!(<ID> "host id"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("delete host")
                .arg(arg!(<ID> "host id"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("list hosts"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    // TODO: Can this be made generic?
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.token.parse()?;

    let mut client = HostClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let id = add_match
                .value_of("ID")
                .expect("Host name expected")
                .to_string();

            let labels = add_match
                .values_of("label")
                .expect("At least one label expected");

            let labels = labels.map(|s| s.to_string()).collect();

            let request = tonic::Request::new(HostMessage {
                id: id.clone(),
                labels,
            });

            client.create(request).await?;

            tracing::info!("host '{id}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("Host id expected");
            let request = tonic::Request::new(DeleteHostRequest { id: id.to_string() });

            client.delete(request).await?;

            tracing::info!("host '{id}' deleted");

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListHostsRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .hosts
                .into_iter()
                .map(|host| vec![host.id.to_string(), host.labels.join(", ")])
                .collect();

            if table_data.is_empty() {
                tracing::info!("no hosts found");

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
