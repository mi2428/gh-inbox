use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "gh inbox",
    bin_name = "gh inbox",
    version,
    about = "Manage GitHub notifications from the inbox.",
    disable_help_subcommand = true,
    propagate_version = true,
    after_help = "\
Examples:
  gh inbox list
  gh inbox sweep
  gh inbox sweep --read
  gh inbox sweep --include-authored
  gh inbox sweep --closed --repo cli/cli --user monalisa
  gh inbox sweep --team-mentioned --no-mentioned
"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List notifications that are still in the inbox.
    List,
    /// Mark matching notifications as done.
    Sweep(SweepArgs),
}

#[derive(Debug, Args, Clone, PartialEq, Eq)]
pub struct SweepArgs {
    /// Only sweep notifications that are already marked as read.
    #[arg(long)]
    pub read: bool,

    /// Only sweep pull request notifications whose pull requests are closed or merged.
    #[arg(long)]
    pub closed: bool,

    /// Only sweep notifications from the given repository.
    #[arg(long, value_name = "OWNER/REPO")]
    pub repo: Option<String>,

    /// Only sweep pull request notifications opened by the given user.
    #[arg(long, value_name = "LOGIN")]
    pub user: Option<String>,

    /// Only sweep notifications whose reason is team_mention.
    #[arg(long)]
    pub team_mentioned: bool,

    /// Only sweep notifications where the reason is not mention.
    #[arg(long)]
    pub no_mentioned: bool,

    /// Also sweep pull request notifications authored by the authenticated user.
    #[arg(long)]
    pub include_authored: bool,
}
