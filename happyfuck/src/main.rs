use std::fs;

use clap::Parser;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::args::Args;
use crate::language::runtime::Runtime;

mod args;
mod language;
mod shell;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    #[cfg(debug_assertions)]
    if args.debug {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .with(EnvFilter::new("off,hf=trace"))
            .init();
    }

    let mut runtime = Runtime::new();

    if let Some(path) = args.path {
        let code = fs::read_to_string(path)?;

        runtime.run(&code).unwrap();
    } else if let Some(code) = args.code {
        runtime.run(&code).unwrap();
    } else {
        shell::start(&mut runtime);
    }

    Ok(())
}
