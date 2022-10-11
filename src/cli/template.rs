use ascii_table::{Align, AsciiTable};
use clap::{arg, Arg, Command};
use fabriq_core::{
    common::TemplateIdRequest, template::template_client::TemplateClient, ListTemplatesRequest,
    TemplateMessage,
};
use tonic::transport::Channel;
use tonic::Request;

use crate::context::Context;

pub fn args() -> Command<'static> {
    Command::new("template")
        .long_flag("template")
        .about("Manage templates")
        .subcommand(
            Command::new("create")
                .about("Create template")
                .arg(
                    Arg::new("repo")
                        .long("repo")
                        .help("Git repository that contains template")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("ref")
                        .long("ref")
                        .help("Git ref that contains template")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(
                    Arg::new("path")
                        .short('p')
                        .long("path")
                        .help("Template git repo path to template")
                        .takes_value(true)
                        .multiple_values(false),
                )
                .arg(arg!(<ID> "Template ID"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete template")
                .arg(arg!(<ID> "ID of template"))
                .arg_required_else_help(true),
        )
        .subcommand(Command::new("list").about("List templates"))
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    let channel = Channel::from_static(context.endpoint).connect().await?;

    let token = context.make_token()?;

    let mut client = TemplateClient::with_interceptor(channel, move |mut req: Request<()>| {
        req.metadata_mut().insert("authorization", token.clone());
        Ok(req)
    });

    match model_match.subcommand() {
        Some(("create", add_match)) => {
            let id = add_match
                .value_of("ID")
                .expect("Template name expected")
                .to_string();
            let repository = add_match
                .value_of("repo")
                .expect("Repo URL expected")
                .to_string();
            let git_ref = add_match.value_of("ref").unwrap_or("main").to_string();
            let path = add_match.value_of("path").unwrap_or("./").to_string();

            let request = tonic::Request::new(TemplateMessage {
                id: id.clone(),
                repository,
                git_ref,
                path,
            });

            client.upsert(request).await?;

            tracing::info!("template '{id}' created");

            Ok(())
        }
        Some(("delete", create_match)) => {
            let id = create_match.value_of("ID").expect("Template id expected");
            let request = tonic::Request::new(TemplateIdRequest {
                template_id: id.to_string(),
            });

            client.delete(request).await?;

            tracing::info!("template '{id}' deleted");

            Ok(())
        }
        Some(("list", _)) => {
            let request = tonic::Request::new(ListTemplatesRequest {});

            let response = client.list(request).await?;

            let table_data: Vec<Vec<String>> = response
                .into_inner()
                .templates
                .into_iter()
                .map(|template| {
                    vec![
                        template.id.to_string(),
                        template.repository.clone(),
                        template.git_ref.clone(),
                        template.path,
                    ]
                })
                .collect();

            if table_data.is_empty() {
                tracing::info!("no templates found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("REPO")
                .set_align(Align::Left);

            ascii_table
                .column(2)
                .set_header("BRANCH")
                .set_align(Align::Left);

            ascii_table
                .column(3)
                .set_header("PATH")
                .set_align(Align::Left);

            ascii_table.print(table_data);

            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
