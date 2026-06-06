use std::time::Instant;

use rustyline::history::FileHistory;
use rustyline::{Config, EditMode, Editor};

use crate::runtime::Runtime;

#[derive(Default)]
struct Shell {
    timing: bool,
}

/// Starts a shell.
///
/// Arguments:
/// * `runtime` - The runtime to use for this shell.
pub fn start(runtime: &mut Runtime) {
    println!("Welcome to the brainfuck shell!");
    println!(
        "Made by Nyeki - Version {} - MIT License",
        env!("CARGO_PKG_VERSION")
    );
    println!("Use $help to display help, $quit to exit the shell.");

    let mut shell = Shell::default();

    let config = Config::builder()
        .auto_add_history(true)
        .edit_mode(EditMode::Emacs)
        .build();
    let mut readline: Editor<(), FileHistory> = Editor::with_config(config).unwrap();

    let _ = readline.load_history("~/.brainfuck-history.txt");

    loop {
        let input = if runtime.nesting == 0 {
            readline.readline(">>> ").unwrap()
        } else {
            readline
                .readline(&format!("[{}> ", runtime.nesting))
                .unwrap()
        };

        readline.add_history_entry(&input).unwrap();
        let _ = readline.save_history("~/.brainfuck-history.txt");

        match input.trim() {
            "$quit" | "$q" => break,
            "$help" | "$h" => command_help(),
            "$code" | "$o" => command_code(runtime),
            "$cell" | "$c" => command_cell(runtime),
            "$memory" | "$m" => command_memory(runtime),
            "$reset" | "$r" => command_reset(runtime),
            "$timing" | "$t" => command_timing(&mut shell),
            _ => {
                let start = Instant::now();

                for command in input.trim().chars() {
                    runtime.execute(command);
                }

                if shell.timing {
                    println!("Took {:?}", start.elapsed());
                }
            }
        }
    }
}

fn command_help() {
    println!(
        "Nyeki's Brainfuck Shell - Version {}",
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
    let mut code = String::with_capacity(runtime.code.len());

    for ch in &runtime.code {
        code.push(*ch);
    }

    println!("{code}");
}

fn command_cell(runtime: &Runtime) {
    println!("Cell {}: {}", runtime.memory_pointer, runtime.read())
}

fn command_memory(runtime: &Runtime) {
    for (i, cell) in runtime.memory.iter().enumerate() {
        if (i + 1) % 10 == 0 {
            println!();
        }

        print!("{cell:<3} ");
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
