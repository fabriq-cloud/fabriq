use akira_core::workload::workload_client::WorkloadClient;
use akira_core::{ListWorkloadsRequest, WorkloadIdRequest, WorkloadMessage};
use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use tonic::metadata::MetadataValue;
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("workload")
        .long_flag("workload")
        .about("manage workloads")
        .subcommand(
            Command::new("create")
                .about("Create workload")
                .arg(
                    Arg::new("team")
                        .short('w')
                        .long("team")
                        .help("team this workload belongs to")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("template")
                        .short('t')
                        .long("template")
                        .help("template this workload should use")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(arg!(<NAME> "workload name"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("delete workload")
                .arg(arg!(<ID> "id of workload"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("init")
                .about("initialize service")
                .arg(
                    Arg::new("seed")
                        .short('s')
                        .long("seed")
                        .help("Seed this workload should be initialized from")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(arg!(<ID> "Workload ID"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("list workloads"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token: MetadataValue<_> = context.profile.pat.parse()?;

    let mut client = WorkloadClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let workload_name = add_match
                .value_of("NAME")
                .expect("workload name expected")
                .to_string();
            let team_id = add_match
                .value_of("team")
                .expect("team id expected")
                .to_string();
            let template_id = add_match
                .value_of("template")
                .expect("template id expected")
                .to_string();

            let request = tonic::Request::new(WorkloadMessage {
                id: WorkloadMessage::make_id(&team_id, &workload_name),
                name: workload_name.clone(),
                team_id,
                template_id,
            });

            client.create(request).await?;

            tracing::info!("workload '{workload_name}' created");

            Ok(())
        }
        Some(("init", init_match)) => {
            let id = init_match
                .value_of("ID")
                .expect("Workload name expected")
                .to_string();
            let seed = init_match
                .value_of("seed")
                .expect("Seed expected")
                .to_string();

            let seed_parts = seed.split('/').collect::<Vec<_>>();

            if seed_parts.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Invalid seed format: Expected Github org/repo."
                ));
            }

            let octocrab = octocrab::OctocrabBuilder::new()
                .personal_token(context.profile.pat.clone())
                .build()?;

            let user = octocrab.current().user().await?;

            octocrab
                .repos(seed_parts[0], seed_parts[1])
                .generate(&id)
                .owner(user.login)
                .include_all_branches(true)
                .private(true)
                .send()
                .await?;

            tracing::info!("workload initialized");

            Ok(())
        }
        Some(("delete", delete_match)) => {
            let id = delete_match.value_of("ID").expect("workload id expected");
            let request = tonic::Request::new(WorkloadIdRequest {
                workload_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("workload '{id}' deleted");

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
                        workload.name.to_string(),
                        workload.team_id.clone(),
                        workload.template_id,
                    ]
                })
                .collect();

            if table_data.is_empty() {
                tracing::info!("no workloads found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(0)
                .set_header("NAME")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("TEAM ID")
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
