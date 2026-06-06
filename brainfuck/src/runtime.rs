use std::io::{self, Read, Write};

const COMMANDS: [char; 8] = ['+', '-', '<', '>', '[', ']', '.', ','];

#[derive(Default)]
pub struct Runtime {
    pub memory: Vec<u8>,
    pub memory_pointer: usize,

    /// The current brackets nesting level, one per bracket.
    pub nesting: usize,

    /// Code buffer keeping potentially-reused code for loops.
    pub code: Vec<char>,
    pub code_pointer: usize,
}

impl Runtime {
    /// Creates a new [`Runtime`].
    ///
    /// Returns:
    /// [`Runtime`] - The newly created runtime.
    pub fn new() -> Self {
        Runtime {
            ..Default::default()
        }
    }

    /// Executes a single command.
    ///
    /// Note that if this is an ending bracket, this will block until the loop finishes executing.
    ///
    /// Arguments:
    /// * `command` - The command to execute.
    pub fn execute(&mut self, command: char) {
        if COMMANDS.contains(&command) {
            self.code.push(command);

            tracing::trace!(
                code = %command,
                command_pointer = %self.code_pointer,
                "Pushed command to code buffer"
            );

            match command {
                '[' => self.nesting += 1,
                ']' => self.nesting -= 1,
                _ => {}
            }

            if self.nesting == 0 {
                match command {
                    '+' => self.execute_command_plus(),
                    '-' => self.execute_command_minus(),
                    '<' => self.execute_command_left(),
                    '>' => self.execute_command_right(),
                    '[' => self.execute_command_left_bracket(),
                    ']' => self.execute_command_right_bracket(self.code.len() - 1),
                    '.' => self.execute_command_dot(),
                    ',' => self.execute_command_comma(),
                    _ => {}
                }
            }
        }
    }

    /// Ensures there's enough memory allocated to use the current pointer.
    ///
    /// For example:
    /// ```
    /// let mut runtime = Runtime::new();
    ///
    /// assert_eq!(runtime.memory.len(), 0);
    ///
    /// runtime.execute('+');
    /// runtime.execute('+');
    ///
    /// runtime.ensure_memory();
    ///
    /// assert_eq!(runtime.memory.len(), 2);
    /// ```
    ///
    /// It will not do anything if there's enough memory allocated to use the current pointer.
    fn ensure_memory(&mut self) {
        if self.memory.len() < self.memory_pointer + 1 {
            let missing = self.memory_pointer + 1 - self.memory.len();

            self.memory.extend(vec![0; missing]);

            tracing::trace!(bytes_added = %missing, "Extended memory.");
        }
    }

    /// Removes all trailing cells with 0s.
    fn shrink_memory(&mut self) {
        let mut i = 0;

        while self.memory.last() == Some(&0) {
            self.memory.pop();
            i += 1;
        }

        tracing::trace!(removed_bytes = i, "Removed trailing 0s off memory.");
    }

    /// Reads the current memory cell.
    ///
    /// Returns:
    /// [`u8`] - The current memory cell's value.
    pub fn read(&self) -> u8 {
        if self.memory.len() < self.memory_pointer + 1 {
            0
        } else {
            self.memory[self.memory_pointer]
        }
    }

    fn write(&mut self, value: u8) {
        if value == 0 {
            if self.memory.len() > self.memory_pointer {
                self.memory[self.memory_pointer] = value;
            }

            self.shrink_memory();
        } else {
            self.ensure_memory();
            self.memory[self.memory_pointer] = value;
        }
    }

    /// Wrapping-adds 1 to the current pointe-at memory cell.
    fn execute_command_plus(&mut self) {
        let old_value = self.read();
        let new_value = old_value.wrapping_add(1);

        self.write(new_value);

        tracing::trace!(old_value, new_value, "Executed + command.");
    }

    /// Wrapping-subtracts 1 to the current pointe-at memory cell.
    fn execute_command_minus(&mut self) {
        let old_value = self.read();
        let new_value = old_value.wrapping_sub(1);

        self.write(new_value);

        tracing::trace!(old_value, new_value, "Executed - command.");
    }

    /// Moves the pointer to the left by one cell, or does nothing if the pointer is 0.
    fn execute_command_left(&mut self) {
        let old_value = self.memory_pointer;
        let new_value = old_value.saturating_sub(1);

        self.memory_pointer = new_value;

        tracing::trace!(old_value, new_value, "Executed < command.");
    }

    /// Moves the pointer to the right by one cell.
    fn execute_command_right(&mut self) {
        let old_value = self.memory_pointer;
        let new_value = old_value.saturating_add(1);

        self.memory_pointer = new_value;

        tracing::trace!(old_value, new_value, "Executed > command.");
    }

    fn execute_command_left_bracket(&mut self) {
        tracing::trace!("Saw [ command.");
    }

    fn execute_command_right_bracket(&mut self, starting_pos: usize) {
        tracing::trace!("Saw ] command.");

        if self.read() != 0 {
            tracing::trace!("Current cell is not 0, executing loop.");

            let mut inverted_pointer = 0;
            let mut brackets_to_skip = 1;

            loop {
                inverted_pointer += 1;
                let command = self.code.get(starting_pos - 1 - inverted_pointer).unwrap();

                match command {
                    '[' => brackets_to_skip -= 1,
                    ']' => brackets_to_skip += 1,
                    _ => {}
                }

                if brackets_to_skip == 0 {
                    break;
                }
            }

            tracing::trace!(
                index = starting_pos - 1 - inverted_pointer,
                "Found matching [."
            );

            let executable_code =
                self.code[starting_pos - inverted_pointer..starting_pos].to_vec();
            let executable_code_display = executable_code.iter().cloned().collect::<String>();

            tracing::trace!(
                code = executable_code_display,
                "A subset of the previous code will be executed."
            );

            while self.read() != 0 {
                let mut nesting = 0;

                for (i, command) in executable_code.iter().enumerate() {
                    match command {
                        '[' => nesting += 1,
                        ']' => nesting -= 1,
                        _ => {}
                    }

                    if nesting == 0 {
                        match command {
                            '+' => self.execute_command_plus(),
                            '-' => self.execute_command_minus(),
                            '<' => self.execute_command_left(),
                            '>' => self.execute_command_right(),
                            '[' => self.execute_command_left_bracket(),
                            ']' => self.execute_command_right_bracket(
                                starting_pos - inverted_pointer + i,
                            ),
                            '.' => self.execute_command_dot(),
                            ',' => self.execute_command_comma(),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Outputs the current cell's byte via STDOUT.
    fn execute_command_dot(&mut self) {
        io::stdout().write_all(&[self.read()]).unwrap();
        io::stdout().flush().unwrap();
    }

    /// Reads one byte from STDIN and writes it to the current cell.
    ///
    /// This command doesn't fail if nothing is read.
    fn execute_command_comma(&mut self) {
        let mut buffer = [0u8];

        let mut stdin = io::stdin();
        if stdin.read(&mut buffer).unwrap() == 1 {
            self.write(buffer[0]);
        };
    }
}
