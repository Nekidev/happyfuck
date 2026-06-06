use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Args {
    /// The path to the happyfuck code file to execute.
    pub path: Option<PathBuf>,

    /// A code string to execute.
    #[arg(short, long)]
    pub code: Option<String>,

    /// Enable debug logging.
    #[cfg(debug_assertions)]
    #[arg(short, long)]
    pub debug: bool,
}
