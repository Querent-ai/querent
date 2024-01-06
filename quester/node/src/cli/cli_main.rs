use anyhow::{bail, Context};
use clap::{arg, Arg, ArgAction, ArgMatches, Command};
use tracing::Level;

use crate::cli::service::{build_serve_command, Serve};

pub fn build_cli() -> Command {
	Command::new("Querent")
        .arg(
            Arg::new("no-color")
                .long("no-color")
                .help(
                    "Disable ANSI terminal codes (colors, etc...) being injected into the logging \
                     output",
                )
                .env("NO_COLOR")
                .value_parser(clap::builder::FalseyValueParser::new())
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(arg!(-y --"yes" "Assume \"yes\" as an answer to all prompts and run non-interactively.")
            .global(true)
            .required(false)
        )
        .subcommand(build_serve_command().display_order(1))
        .arg_required_else_help(true)
        .disable_help_subcommand(true)
        .subcommand_required(true)
}

#[derive(Debug, PartialEq)]
pub enum CliCommand {
	Serve(Serve),
}

impl CliCommand {
	pub fn default_log_level(&self) -> Level {
		match self {
			CliCommand::Serve(_) => Level::INFO,
		}
	}

	pub fn parse_cli_args(mut matches: ArgMatches) -> anyhow::Result<Self> {
		let (subcommand, submatches) =
			matches.remove_subcommand().context("failed to parse command")?;
		match subcommand.as_str() {
			"serve" => Serve::parse_cli_args(submatches).map(CliCommand::Serve),
			_ => bail!("unknown command `{subcommand}`"),
		}
	}

	pub async fn execute(self) -> anyhow::Result<()> {
		match self {
			CliCommand::Serve(subcommand) => subcommand.execute().await,
		}
	}
}
