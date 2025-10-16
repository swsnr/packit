// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// Licensed under the EUPL-1.2 OR GPL-3.0
//
// See https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12

use clap::{Args, Parser, Subcommand};

const AFTER_LONG_HELP: &str = "\
Automatically print colored output if stdout is a TTY, unless overridden by
environment variables as follows:

- If $NO_COLOR is set to a non-empty string, never print any colors.
- If $CLICOLOR_FORCE is set to a non-empty string, always print colors even if
  stdout is not a TTY.
";

const LONG_VERSION: &str = concat!(
    env!("CARGO_BIN_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    "\n",
    env!("CARGO_PKG_HOMEPAGE"),
    "\nLicense: ",
    env!("CARGO_PKG_LICENSE"),
);

/// Analyse pacman dependency graphs.
#[derive(Debug, Parser)]
#[command(version, about, after_long_help = AFTER_LONG_HELP, long_version = LONG_VERSION)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Dependents(Dependents),
    Orphans(Orphans),
    #[cfg(feature = "completions")]
    Completions(Completions),
}

/// List orphan packages.
#[derive(Args, Debug)]
pub struct Orphans {
    #[clap(flatten)]
    pub graph_options: GraphOptions,
}

/// List packages which depend on a package.
#[derive(Args, Debug)]
pub struct Dependents {
    /// The package whose installation reason to find.
    pub package: String,
    #[clap(flatten)]
    pub graph_options: GraphOptions,
}

#[derive(Debug, Args)]
/// Options for package graphs.
pub struct GraphOptions {
    /// Ignore optional dependencies.
    #[clap(long)]
    pub ignore_optdepends: bool,
    /// Show less information.
    #[clap(short = 'q', long = "quiet")]
    pub quiet: bool,
    /// Render the graph as dot.
    #[clap(long)]
    pub dot: bool,
}

/// Generate shell completions.
#[derive(Args, Debug)]
#[cfg(feature = "completions")]
pub struct Completions {
    /// The shell to generate completions for.
    pub shell: clap_complete::Shell,
}

#[cfg(feature = "completions")]
impl Completions {
    pub fn print(&self) {
        use clap::CommandFactory;
        clap_complete::generate(
            self.shell,
            &mut CliArgs::command(),
            env!("CARGO_BIN_NAME"),
            &mut std::io::stdout(),
        );
    }
}
