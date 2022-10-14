use ascii_table::{Align, AsciiTable};
use clap::{Arg, ArgAction, Command};

use crate::context::Context;

pub fn args() -> Command {
    Command::new("team")
        .arg_required_else_help(true)
        .about("View teams")
        .subcommand(
            Command::new("list").about("list teams").arg(
                Arg::new("organization")
                    .short('o')
                    .long("organization")
                    .help("organization to query")
                    .action(ArgAction::Set),
            ),
        )
}

pub async fn handlers(
    model_match: &clap::ArgMatches,
    context: &Context<'static>,
) -> anyhow::Result<()> {
    match model_match.subcommand() {
        Some(("list", list_match)) => {
            let organization_id = list_match
                .get_one::<String>("organization")
                .expect("organization id expected")
                .to_string();

            let pat = context.get_pat();

            let octocrab = octocrab::OctocrabBuilder::new()
                .personal_token(pat.clone())
                .build()?;

            let teams = octocrab
                .teams(organization_id.clone())
                .list()
                .per_page(100)
                .send()
                .await?;

            let table_data: Vec<Vec<String>> = teams
                .items
                .into_iter()
                .map(|team| {
                    vec![
                        format!("{}:{}", &organization_id, team.slug),
                        team.name,
                        team.description.unwrap(),
                    ]
                })
                .collect();

            if table_data.is_empty() {
                tracing::info!("no teams found");

                return Ok(());
            }

            let mut ascii_table = AsciiTable::default();

            ascii_table
                .column(0)
                .set_header("ID")
                .set_align(Align::Left);

            ascii_table
                .column(1)
                .set_header("NAME")
                .set_align(Align::Left);

            ascii_table
                .column(2)
                .set_header("DESCRIPTION")
                .set_align(Align::Left);

            ascii_table.print(table_data);
            Ok(())
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
