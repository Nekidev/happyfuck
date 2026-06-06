use std::fs;
use std::time::Instant;

use rustyline::history::FileHistory;
use rustyline::{Config, EditMode, Editor};

use crate::language::parsing::{Nesting, Size};
use crate::language::runtime::Runtime;

const HISTORY_PATH: &str = ".happyfuck-history.txt";

#[derive(Default)]
struct Shell {
    timing: bool,
}

/// Starts a shell.
///
/// Arguments:
/// * `runtime` - The runtime to use for this shell.
pub fn start(runtime: &mut Runtime) {
    println!("Welcome to the happyfuck shell!");
    println!(
        "Made by Nyeki - Version {} - MIT License",
        env!("CARGO_PKG_VERSION")
    );
    println!("Use /help to display help, /quit to exit the shell.");

    let mut shell = Shell::default();

    let config = Config::builder()
        .auto_add_history(true)
        .edit_mode(EditMode::Emacs)
        .build();
    let mut readline: Editor<(), FileHistory> = Editor::with_config(config).unwrap();

    let _ = readline.load_history(HISTORY_PATH);

    loop {
        let input = if runtime.parser.nesting.is_empty() {
            readline.readline(">>> ").unwrap()
        } else {
            let mut repr = String::with_capacity(runtime.parser.nesting.len());

            for kind in runtime
                .parser
                .nesting
                .iter()
                .skip(runtime.parser.nesting.len().saturating_sub(2))
            {
                match kind {
                    Nesting::Braces => repr.push('{'),
                    Nesting::Brackets => repr.push('['),
                    Nesting::Parentheses => repr.push('('),
                }
            }

            readline.readline(&format!("{repr:<2}> ")).unwrap()
        };

        let mut write_to_history = true;

        match input.trim() {
            "/quit" | "/q" => break,
            "/help" | "/h" => command_help(),
            "/code" | "/o" => command_code(runtime),
            "/cell" | "/c" => command_cell(runtime),
            "/memory" | "/m" => command_memory(runtime),
            "/reset" | "/r" => command_reset(runtime),
            "/timing" | "/t" => command_timing(&mut shell),
            _ => {
                let start = Instant::now();

                let result = runtime.run(&input);

                let elapsed = start.elapsed();

                if runtime.last_output.is_some() && runtime.last_output != Some('\n') {
                    // readline.readline() clears the current line, output is lost if it doesn't
                    // end in a new line.
                    println!();
                }

                if let Err(error) = result
                    && error.is_fatal
                {
                    println!("{error}");
                    write_to_history = false;
                }

                if shell.timing {
                    println!("Took {elapsed:?}");
                }
            }
        }

        if write_to_history {
            if !fs::exists(HISTORY_PATH).unwrap() {
                let _ = fs::write(HISTORY_PATH, "");
            }

            readline.add_history_entry(&input).unwrap();
            let _ = readline.save_history(HISTORY_PATH);
        }
    }
}

fn command_help() {
    println!(
        "Nyeki's Happyfuck Shell - Version {}",
        env!("CARGO_PKG_VERSION")
    );
    println!("MIT License");
    println!();
    println!("$h, $help   - Displays this command.");
    println!("$q, $quit   - Quits the shell.");
    println!("$o, $code   - Displays all the code executed in this session.");
    println!("$c, $cell   - Displays the current cell's value.");
    println!("$m, $memory - Displays the current memory.");
    println!("$r, $reset  - Resets the current session.");
    println!("$t, $timing - Display the time each line takes to execute.");
}

fn command_code(runtime: &Runtime) {
    println!("{}", runtime.code);
}

fn command_cell(runtime: &Runtime) {
    println!(
        "Cell {}: {}",
        runtime.memory_pointer,
        runtime.read(runtime.memory_pointer, Size::Byte)
    )
}

fn command_memory(runtime: &Runtime) {
    for (i, cell) in runtime.memory.iter().enumerate() {
        if (i) % 10 == 0 && i != 0 {
            println!();
        }

        print!("{cell:0>3} ");
    }

    println!();
}

fn command_reset(runtime: &mut Runtime) {
    *runtime = Runtime::new();
    println!("The session was reset. All memory was cleared and the pointer is at the first cell.");
}

fn command_timing(shell: &mut Shell) {
    shell.timing = !shell.timing;

    if shell.timing {
        println!("Timing display was enabled.");
    } else {
        println!("Timing display was disabled.");
    }
}
