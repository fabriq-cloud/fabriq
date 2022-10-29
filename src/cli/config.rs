use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, ArgAction, Command};
use fabriq_core::{
    config::config_client::ConfigClient, ConfigIdRequest, ConfigMessage, ConfigValueType,
    QueryConfigRequest,
};
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command {
    Command::new("config")
        .arg_required_else_help(true)
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
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("template")
                        .long("template")
                        .help("owning template id")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("workload")
                        .long("workload")
                        .help("owning workload id")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("team")
                        .long("team")
                        .help("owning team")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("type")
                        .long("type")
                        .help("value of type (default 'string')")
                        .action(ArgAction::Set),
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
                    Arg::new("deployment")
                        .short('d')
                        .long("deployment")
                        .help("Deployment to query config for")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("template")
                        .short('t')
                        .long("template")
                        .help("Template to query config for")
                        .action(ArgAction::Set),
                )
                .arg(
                    Arg::new("workload")
                        .short('w')
                        .long("workload")
                        .help("Workload to query config for")
                        .action(ArgAction::Set),
                ),
        )
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token = context.make_token()?;

    let mut client = ConfigClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", create_match)) => {
            let key = create_match
                .get_one::<String>("KEY")
                .expect("config key expected")
                .to_string();

            let value = create_match
                .get_one::<String>("VALUE")
                .expect("config value expected")
                .to_string();

            let template_id = create_match.get_one::<String>("template");
            let workload_id = create_match.get_one::<String>("workload");
            let deployment_id = create_match.get_one::<String>("deployment");
            let value_type_option = create_match.get_one::<String>("type");

            let value_type = if let Some(value_type) = value_type_option {
                if value_type == "keyvalue" {
                    ConfigValueType::KeyValueType as i32
                } else if value_type == "string" {
                    ConfigValueType::StringType as i32
                } else {
                    return Err(anyhow::anyhow!("Invalid config value type"));
                }
            } else {
                ConfigValueType::StringType as i32
            };

            let owning_model = match deployment_id {
                Some(deployment_id) => {
                    ConfigMessage::make_owning_model("deployment", deployment_id)?
                }
                None => match workload_id {
                    Some(workload_id) => ConfigMessage::make_owning_model("workload", workload_id)?,
                    None => match template_id {
                        Some(template_id) => {
                            ConfigMessage::make_owning_model("template", template_id)?
                        }
                        None => {
                            panic!("owning workload, template, or deployment id must be specified")
                        }
                    },
                },
            };

            let id = ConfigMessage::make_id(&owning_model, &key);

            let request = tonic::Request::new(ConfigMessage {
                id: id.clone(),
                owning_model,

                key,
                value,

                value_type,
            });

            client.upsert(request).await?;

            tracing::info!("config '{id}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match
                .get_one::<String>("ID")
                .expect("Config id expected");

            let request = tonic::Request::new(ConfigIdRequest {
                config_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("config '{id}' deleted");

            Ok(())
        }
        Some(("query", list_match)) => {
            let deployment_id = list_match.get_one::<String>("deployment");
            let template_id = list_match.get_one::<String>("template");
            let workload_id = list_match.get_one::<String>("workload");

            let request = if let Some(deployment_id) = deployment_id {
                tonic::Request::new(QueryConfigRequest {
                    model_name: "deployment".to_string(),
                    model_id: deployment_id.to_string(),
                })
            } else if let Some(workload_id) = workload_id {
                tonic::Request::new(QueryConfigRequest {
                    model_name: "workload".to_string(),
                    model_id: workload_id.to_string(),
                })
            } else if let Some(template_id) = template_id {
                tonic::Request::new(QueryConfigRequest {
                    model_name: "template".to_string(),
                    model_id: template_id.to_string(),
                })
            } else {
                panic!("owning workload, template, or deployment id must be specified")
            };

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
