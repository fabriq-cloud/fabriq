use akira_core::config::config_client::ConfigClient;
use akira_core::{ConfigIdRequest, ConfigMessage, ConfigValueType, QueryConfigRequest};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("config")
        .long_flag("config")
        .about("manage configs")
        .subcommand(
            Command::new("create")
                .about("create config")
                .arg(
                    Arg::new("deployment")
                        .short('d')
                        .long("deployment")
                        .help("owning deployment id")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("template")
                        .short('t')
                        .long("template")
                        .help("owning template id")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("workload")
                        .short('w')
                        .long("workload")
                        .help("owning workload id")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("type")
                        .long("type")
                        .help("value of type (default 'string')")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(arg!(<KEY> "Config key"))
                .arg(arg!(<VALUE> "Config value"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("delete config")
                .arg(arg!(<ID> "ID of config"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("query")
                .about("query configs")
                .arg(
                    Arg::new("workload")
                        .short('w')
                        .long("workload")
                        .help("Workload to query config for")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("deployment")
                        .short('d')
                        .long("deployment")
                        .help("Deployment to query config for")
                        .takes_value(true)
                        .multiple_values(false),
                ),
        )
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.token.parse()?;

    let mut client = ConfigClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", create_match)) => {
            let key = create_match
                .value_of("KEY")
                .expect("Config key expected")
                .to_string();
            let value = create_match
                .value_of("VALUE")
                .expect("Config value expected")
                .to_string();

            let template_id = create_match.value_of("template");
            let workload_id = create_match.value_of("workload");
            let deployment_id = create_match.value_of("deployment");
            let value_type_option = create_match.value_of("type");

            let value_type = match value_type_option {
                Some("keyvalue") => ConfigValueType::KeyValueType as i32,
                Some("string") => ConfigValueType::StringType as i32,
                None => ConfigValueType::StringType as i32,
                _ => return Err(anyhow::anyhow!("Invalid value type")),
            };

            let owning_model = match workload_id {
                Some(workload_id) => {
                    format!("workload:{}", workload_id)
                }
                None => match deployment_id {
                    Some(deployment_id) => {
                        format!("deployment:{}", deployment_id)
                    }
                    None => match template_id {
                        Some(template_id) => {
                            format!("template:{}", template_id)
                        }
                        None => {
                            panic!("owning workload, template, or deployment id must be specified")
                        }
                    },
                },
            };

            let id = format!("{}|{}", owning_model, key);

            let request = tonic::Request::new(ConfigMessage {
                id: id.clone(),
                owning_model,

                key,
                value,

                value_type,
            });

            client.create(request).await?;

            tracing::info!("config '{id}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("Config id expected");

            let request = tonic::Request::new(ConfigIdRequest {
                config_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("config '{id}' deleted");

            Ok(())
        }
        Some(("query", list_match)) => {
            let workload_id = list_match.value_of("workload").expect("workload expected");
            let deployment_id = list_match
                .value_of("deployment")
                .expect("deployment expected");

            let request = tonic::Request::new(QueryConfigRequest {
                workload_id: workload_id.to_string(),
                deployment_id: deployment_id.to_string(),
            });

            let response = client.query(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .configs
                .into_iter()
                .map(|config| {
                    vec![
                        config.id.to_string(),
                        config.owning_model.to_string(),
                        config.key.to_string(),
                        config.value,
                    ]
                })
                .collect();

            if table_data.is_empty() {
                tracing::info!("no configs found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("OWNER")
                .set_align(Align::Left);

            ascii_table
                .column(2)
                .set_header("KEY")
                .set_align(Align::Left);

            ascii_table
                .column(3)
                .set_header("VALUE")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
