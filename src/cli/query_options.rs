use std::ffi::OsString;

#[derive(Debug, Default, clap::Parser)]
#[command(
    no_binary_name = true,
    disable_help_flag = true,
    disable_version_flag = true
)]
pub(super) struct QueryOptions {
    #[arg(
        long,
        value_parser = ["source", "callable-skeleton"],
        required_unless_present = "help"
    )]
    pub(super) projection: Option<String>,
    #[arg(long)]
    pub(super) selector: Option<String>,
    #[arg(long)]
    pub(super) json: bool,
    #[arg(long, short = 'h')]
    pub(super) help: bool,
    #[arg(long)]
    pub(super) workspace: Option<std::path::PathBuf>,
}

impl QueryOptions {
    pub(super) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        <Self as clap::Parser>::try_parse_from(args).map_err(|error| error.to_string())
    }
}
