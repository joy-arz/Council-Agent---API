use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[allow(non_camel_case_types)]
pub struct cli_args {
    /// the query or topic for the council to discuss
    pub query: Option<String>,

    /// number of rounds for agents
    #[arg(short, long)]
    pub rounds: Option<usize>,

    /// model provider to use (openai or anthropic)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// model name to use
    #[arg(short, long)]
    pub model: Option<String>,

    /// whether to run in server mode
    #[arg(long)]
    pub server: bool,

    /// workspace directory for the project
    #[arg(short, long)]
    pub workspace: Option<std::path::PathBuf>,
}
