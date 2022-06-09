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
                    Arg::new("cpu")
                        .short('c')
                        .long("cpu")
                        .help("cpu capacity of host")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("memory")
                        .short('m')
                        .long("memory")
                        .help("memory capacity of host")
                        .takes_value(true)
                        .multiple_values(false),
                )
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
    let channel = Channel::from_static(&context.endpoint).connect().await?;

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

            let cpu_capacity = add_match
                .value_of("cpu")
                .expect("cpu capacity expected")
                .to_string()
                .parse::<i32>()?;

            let memory_capacity = add_match
                .value_of("memory")
                .expect("memory capacity expected")
                .to_string()
                .parse::<i64>()?;

            println!("host create '{id}'");

            let request = tonic::Request::new(HostMessage {
                id,
                labels,
                cpu_capacity,
                memory_capacity,
            });

            client.create(request).await?;

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("Host id expected");

            println!("host delete '{id}'");

            let request = tonic::Request::new(DeleteHostRequest { id: id.to_string() });

            client.delete(request).await?;

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListHostsRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .hosts
                .into_iter()
                .map(|host| {
                    vec![
                        host.id.to_string(),
                        host.labels.clone().join(", "),
                        host.cpu_capacity.to_string(),
                        host.memory_capacity.to_string(),
                    ]
                })
                .collect();

            if table_data.len() == 0 {
                println!("No hosts found");

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

            ascii_table
                .column(2)
                .set_header("CPU CAPACITY")
                .set_align(Align::Left);

            ascii_table
                .column(3)
                .set_header("MEMORY CAPACITY")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
