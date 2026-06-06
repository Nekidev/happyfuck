use std::fs;

use clap::Parser;

use crate::{args::Args, runtime::Runtime};

mod args;
mod logging;
mod runtime;
mod shell;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    #[cfg(debug_assertions)]
    logging::init(args.logging_level.into());

    let mut runtime = Runtime::new();

    if let Some(path) = args.path {
        let code = fs::read_to_string(path)?;

        for command in code.chars() {
            runtime.execute(command);
        }
    } else if let Some(code) = args.code {
        for command in code.chars() {
            runtime.execute(command);
        }
    } else {
        shell::start(&mut runtime);
    }

    Ok(())
}
