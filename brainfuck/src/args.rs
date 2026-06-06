use std::path::PathBuf;

/// A brainfuck interpreter.
#[derive(clap::Parser)]
pub struct Args {
    /// Path to a brainfuck code file.
    pub path: Option<PathBuf>,

    /// A code string to execute.
    #[arg(long, short)]
    pub code: Option<String>,

    /// The amount of logging to enable. Useful for internal debugging.
    #[cfg(debug_assertions)]
    #[arg(long, short, default_value = "off")]
    pub logging_level: LevelFilter,
}

/// An enum with the different logging levels that can be used in the CLI.
///
/// This enum is used to parse the `--log-level` or `-l` command line argument and then map the
/// value to [`tracing::LevelFilter`].
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum LevelFilter {
    /// No logs at all.
    Off,

    /// The most verbose level, used for debugging purposes.
    Trace,

    /// Events that are useful to be displayed during development, but not in production.
    Debug,

    /// Used for general information about the bot's state and milestones.
    Info,

    /// Used for warnings, when something unexpected happens but the bot can still continue
    /// running.
    Warn,

    /// Used for errors, when something goes wrong and the bot cannot continue running.
    Error,
}

#[allow(clippy::from_over_into)]
impl Into<tracing::level_filters::LevelFilter> for LevelFilter {
    fn into(self) -> tracing::level_filters::LevelFilter {
        match self {
            Self::Off => tracing::level_filters::LevelFilter::OFF,
            Self::Trace => tracing::level_filters::LevelFilter::TRACE,
            Self::Debug => tracing::level_filters::LevelFilter::DEBUG,
            Self::Info => tracing::level_filters::LevelFilter::INFO,
            Self::Warn => tracing::level_filters::LevelFilter::WARN,
            Self::Error => tracing::level_filters::LevelFilter::ERROR,
        }
    }
}
