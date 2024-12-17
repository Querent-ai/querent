// Copyright (C) 2023 QuerentAI LLC.
// This file is part of Querent.

// The Licensed Work is licensed under the Business Source License 1.1 (BSL 1.1).
// You may use this file in compliance with the BSL 1.1, subject to the following restrictions:
// 1. You may not use the Licensed Work for AI-related services, database services,
//    or any service or product offering that provides database, big data, or analytics
//    services to third parties unless explicitly authorized by QuerentAI LLC.
// 2. For more details, see the LICENSE file or visit https://mariadb.com/bsl11/.

// For inquiries about alternative licensing arrangements, please contact contact@querent.xyz.

// The Licensed Work is provided "AS IS", WITHOUT WARRANTY OF ANY KIND, express or implied,
// including but not limited to the warranties of merchantability, fitness for a particular purpose,
// and non-infringement. See the Business Source License for more details.

// This software includes code developed by QuerentAI LLC (https://querent.ai).

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
        .disable_help_subcommand(true)
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
		// Check if no subcommand is provided and default to `serve`
		if matches.subcommand_name().is_none() {
			Serve::parse_cli_args(ArgMatches::default()).map(CliCommand::Serve)
		} else {
			let (subcommand, submatches) = matches.remove_subcommand().unwrap_or_default();
			match subcommand.as_str() {
				"serve" => Serve::parse_cli_args(submatches).map(CliCommand::Serve),
				_ => Serve::parse_cli_args(submatches).map(CliCommand::Serve),
			}
		}
	}

	pub async fn execute(self) -> anyhow::Result<()> {
		match self {
			CliCommand::Serve(subcommand) => subcommand.execute().await,
		}
	}
}
