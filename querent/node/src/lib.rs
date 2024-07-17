pub mod cli;
pub mod serve;
use std::sync::Arc;

use clap::Arg;
use proto::config::DEFAULT_CONFIG_PATH;
pub use serve::*;

pub type EnvFilterReloadFn = Arc<dyn Fn(&str) -> anyhow::Result<()> + Send + Sync>;

fn config_cli_arg() -> Arg {
	Arg::new("config")
		.long("config")
		.help("Config file location")
		.env("QUERENT_CONFIG")
		.default_value(DEFAULT_CONFIG_PATH)
		.global(true)
		.display_order(1)
}
