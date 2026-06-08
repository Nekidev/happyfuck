use std::collections::HashMap;
use std::io::{self, Read, Write};

use tracing::instrument;

use crate::language::errors::SyntaxError;
use crate::language::parsing::{Expression, IfBranch, Parser, Size, Statement};
use crate::language::tokenizing::Tokenizer;

#[derive(Default)]
pub struct Runtime {
    pub memory: Vec<u8>,
    pub memory_pointer: usize,

    pub functions: HashMap<String, Function>,

    pub flag_carry: bool,

    pub code: String,

    pub tokenizer: Tokenizer,
    pub parser: Parser,

    pub last_output: Option<char>,
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            ..Default::default()
        }
    }

    fn get_full_snapshot(&self) -> FullSnapshot {
        FullSnapshot {
            memory: self.memory.clone(),
            memory_pointer: self.memory_pointer,
            functions: self.functions.clone(),
            flag_carry: self.flag_carry,
        }
    }

    fn set_full_snapshot(&mut self, snapshot: FullSnapshot) {
        self.memory = snapshot.memory;
        self.memory_pointer = snapshot.memory_pointer;
        self.functions = snapshot.functions;
        self.flag_carry = snapshot.flag_carry;
    }

    fn get_scope_snapshot(&self) -> ScopeSnapshot {
        ScopeSnapshot {
            functions: self.functions.clone(),
            flag_carry: self.flag_carry,
        }
    }

    fn set_scope_snapshot(&mut self, snapshot: ScopeSnapshot) {
        self.functions = snapshot.functions;
        self.flag_carry = snapshot.flag_carry;
    }

    #[instrument(skip(self), target = "hf::language::runtime::Runtime::read")]
    pub fn read(&self, pointer: usize, size: Size) -> u64 {
        match size {
            Size::None => *self.memory.get(pointer).unwrap_or(&0) as u64,
            Size::Byte => *self.memory.get(pointer).unwrap_or(&0) as u64,
            Size::Word => {
                let b1 = *self.memory.get(pointer).unwrap_or(&0);
                let b2 = *self.memory.get(pointer + 1).unwrap_or(&0);

                u16::from_be_bytes([b1, b2]) as u64
            }
            Size::DWord => {
                let b1 = *self.memory.get(pointer).unwrap_or(&0);
                let b2 = *self.memory.get(pointer + 1).unwrap_or(&0);
                let b3 = *self.memory.get(pointer + 2).unwrap_or(&0);
                let b4 = *self.memory.get(pointer + 3).unwrap_or(&0);

                u32::from_be_bytes([b1, b2, b3, b4]) as u64
            }
            Size::QWord => {
                let b1 = *self.memory.get(pointer).unwrap_or(&0);
                let b2 = *self.memory.get(pointer + 1).unwrap_or(&0);
                let b3 = *self.memory.get(pointer + 2).unwrap_or(&0);
                let b4 = *self.memory.get(pointer + 3).unwrap_or(&0);
                let b5 = *self.memory.get(pointer + 4).unwrap_or(&0);
                let b6 = *self.memory.get(pointer + 5).unwrap_or(&0);
                let b7 = *self.memory.get(pointer + 6).unwrap_or(&0);
                let b8 = *self.memory.get(pointer + 7).unwrap_or(&0);

                u64::from_be_bytes([b1, b2, b3, b4, b5, b6, b7, b8])
            }
        }
    }

    #[instrument(skip(self), target = "hf::language::runtime::Runtime::write")]
    fn write(&mut self, pointer: usize, value: u64, size: Size) {
        let required_size = pointer + size as usize;

        while self.memory.len() < required_size {
            self.memory.push(0);
        }

        match size {
            Size::None => panic!(),
            Size::Byte => {
                *self.memory.get_mut(pointer).unwrap() = value as u8;
            }
            Size::Word => {
                let [_, _, _, _, _, _, b1, b2] = value.to_be_bytes();

                *self.memory.get_mut(pointer).unwrap() = b1;
                *self.memory.get_mut(pointer + 1).unwrap() = b2;
            }
            Size::DWord => {
                let [_, _, _, _, b1, b2, b3, b4] = value.to_be_bytes();

                *self.memory.get_mut(pointer).unwrap() = b1;
                *self.memory.get_mut(pointer + 1).unwrap() = b2;
                *self.memory.get_mut(pointer + 2).unwrap() = b3;
                *self.memory.get_mut(pointer + 3).unwrap() = b4;
            }
            Size::QWord => {
                let [b1, b2, b3, b4, b5, b6, b7, b8] = value.to_be_bytes();

                *self.memory.get_mut(pointer).unwrap() = b1;
                *self.memory.get_mut(pointer + 1).unwrap() = b2;
                *self.memory.get_mut(pointer + 2).unwrap() = b3;
                *self.memory.get_mut(pointer + 3).unwrap() = b4;
                *self.memory.get_mut(pointer + 4).unwrap() = b5;
                *self.memory.get_mut(pointer + 5).unwrap() = b6;
                *self.memory.get_mut(pointer + 6).unwrap() = b7;
                *self.memory.get_mut(pointer + 7).unwrap() = b8;
            }
        }

        self.shrink();
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::shrink")]
    fn shrink(&mut self) {
        let mut bytes = 0;

        while self.memory.last() == Some(&0) {
            self.memory.pop();
            bytes += 1;
        }

        tracing::trace!(bytes, "Shrunk memory.");
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run")]
    pub fn run(&mut self, code: &str) -> Result<(), SyntaxError> {
        self.last_output = None;

        tracing::trace!(code, "Running code...");

        let tokens = self.tokenizer.tokenize(code)?;

        tracing::trace!(?tokens, "Code tokenized.");

        let statements = self.parser.feed(tokens)?;

        tracing::trace!(?statements, "Code parsed.");

        self.code.push_str(code);

        self.run_statements(statements, Size::None);

        tracing::trace!(code, "Finished running code.");

        Ok(())
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_statements")]
    fn run_statements(&mut self, code: Vec<Statement>, size: Size) -> Option<u64> {
        for statement in code {
            match statement {
                Statement::Add {
                    target,
                    size,
                    value,
                } => self.run_add(target, size, value),
                Statement::Subtract {
                    target,
                    size,
                    value,
                } => self.run_subtract(target, size, value),
                Statement::Set {
                    target,
                    size,
                    value,
                } => self.run_set(target, size, value),
                Statement::Left(expr) => self.run_left(expr),
                Statement::Right(expr) => self.run_right(expr),
                Statement::Goto(expr) => self.run_goto(expr),
                Statement::Input { target } => self.run_input(target),
                Statement::Output { size, value } => self.run_output(size, value),
                Statement::DebugOutput { size, value } => self.run_debug_output(size, value),
                Statement::Pointer { target, size } => self.run_pointer(target, size),
                Statement::While {
                    code,
                    expr,
                    is_negated,
                } => {
                    let result = self.run_while(code, expr, is_negated);
                    if result.is_some() {
                        return result;
                    }
                }
                Statement::Repeat(code, expr) => {
                    let result = self.run_repeat(code, expr);
                    if result.is_some() {
                        return result;
                    }
                }
                Statement::FunctionCall { target, name } => {
                    self.run_function_call(target, name);
                }
                Statement::FunctionDefinition { name, code, size } => {
                    self.run_function_def(name, code, size)
                }
                Statement::Return { value } => return Some(self.run_return(value, size)),
                Statement::FlagCarry { target } => self.run_flag_carry(target),
                Statement::If(branches) => self.run_if(branches),
            }
        }

        None
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_expression")]
    fn run_expression(&mut self, code: Vec<Statement>, size: Size) -> u64 {
        tracing::trace!(?size, "Computing code expression...");

        let snapshot = self.get_full_snapshot();

        self.run_statements(code, Size::None);

        let value = self.read(self.memory_pointer, size);

        tracing::trace!(value, ?size, "Computed code expression.");

        self.set_full_snapshot(snapshot);

        value
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_target")]
    fn run_target(&mut self, expr: Option<Expression>) -> usize {
        match expr {
            None => self.memory_pointer,
            Some(Expression::Code(code, size)) => self.run_expression(code, size) as usize,
            Some(Expression::Fixed(amount)) => amount as usize,
            Some(Expression::Size(size)) => {
                self.read(self.memory_pointer, size.or(Size::Byte)) as usize
            }
            Some(Expression::None) => self.read(self.memory_pointer, Size::Byte) as usize,
            Some(Expression::Function(name)) => self.run_function_call(None, name) as usize,
            Some(Expression::String(_)) => unreachable!(),
        }
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_add")]
    fn run_add(&mut self, target: Option<Expression>, mut size: Size, value: Expression) {
        let amount: u64;

        let target = self.run_target(target);

        match value {
            Expression::None => {
                amount = 1;

                tracing::trace!(amount, ?size, "Adding implicit 1...");
            }
            Expression::Fixed(famount) => {
                amount = famount;

                tracing::trace!(amount, ?size, "Adding fixed value...");
            }
            Expression::Code(code, rsize) => {
                amount = self.run_expression(code, rsize);
                size = size.or(rsize);

                tracing::trace!(amount, ?size, "Adding computed value...");
            }
            Expression::Function(name) => {
                amount = self.run_function_call(None, name);

                tracing::trace!(amount, ?size, "Adding returned value...");
            }
            Expression::String(_) => unreachable!(),
            Expression::Size(_) => unreachable!(),
        };

        size = size.or(Size::Byte);

        let value = self.read(target, size);
        let (result, is_overflowing) = size.overflowing_add(value, amount);

        self.write(target, result, size);
        self.flag_carry = is_overflowing;

        tracing::trace!("Run + command.");
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::sun_subtract")]
    fn run_subtract(&mut self, target: Option<Expression>, mut size: Size, value: Expression) {
        let amount: u64;

        let target = self.run_target(target);

        match value {
            Expression::None => {
                amount = 1;

                tracing::trace!(amount, ?size, "Subtracting implicit 1...");
            }
            Expression::Fixed(famount) => {
                amount = famount;

                tracing::trace!(amount, ?size, "Subtracting fixed value...");
            }
            Expression::Code(code, rsize) => {
                amount = self.run_expression(code, rsize);
                size = size.or(rsize);

                tracing::trace!(amount, ?size, "Subtracting computed value...");
            }
            Expression::Function(name) => {
                let result = self.run_function_call(None, name);

                if size.is_some() {
                    amount = size.wrap(result);
                } else {
                    amount = result;
                }

                tracing::trace!(amount, ?size, "Subtracting returned value...");
            }
            Expression::String(_) => unreachable!(),
            Expression::Size(_) => unreachable!(),
        };

        size = size.or(Size::Byte);

        let value = self.read(target, size);
        let (result, is_overflowing) = size.overflowing_sub(value, amount);

        self.write(target, result, size);
        self.flag_carry = is_overflowing;
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_set")]
    fn run_set(&mut self, target: Option<Expression>, mut size: Size, value: Expression) {
        let amount: u64;

        let mut target = self.run_target(target);

        match value {
            Expression::Fixed(famount) => {
                amount = famount;

                tracing::trace!(amount, ?size, "Setting fixed value...");
            }
            Expression::Code(code, rsize) => {
                amount = self.run_expression(code, rsize);
                size = size.or(rsize);

                tracing::trace!(amount, ?size, "Setting computed value...");
            }
            Expression::String(content) => {
                if size.is_some() {
                    self.write(target, content.len() as u64, size);
                    target += size as usize;
                }

                for ch in content.chars() {
                    self.write(target, ch as u64, Size::Byte);
                    target += 1;
                }

                tracing::trace!(content, ?size, "Setting string...");
                return;
            }
            Expression::Function(name) => {
                let result = self.run_function_call(None, name);

                if size.is_some() {
                    amount = size.wrap(result);
                } else {
                    amount = result;
                }

                tracing::trace!(amount, ?size, "Setting returned value...");
            }
            Expression::None => unreachable!(),
            Expression::Size(_) => unreachable!(),
        };

        size = size.or(Size::Byte);

        self.write(target, amount, size);
        self.flag_carry = size.fits(amount);
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_left")]
    fn run_left(&mut self, expr: Expression) {
        let amount: u64;

        match expr {
            Expression::None => {
                amount = 1;

                tracing::trace!("Moving left by 1...");
            }
            Expression::Fixed(famount) => {
                amount = famount;

                tracing::trace!(amount, "Moving left by fixed value...");
            }
            Expression::Code(code, size) => {
                amount = self.run_expression(code, size);

                tracing::trace!(amount, ?size, "Moving left by computed value...");
            }
            Expression::Function(name) => {
                amount = self.run_function_call(None, name);

                tracing::trace!(amount, "Moving left by returned value...");
            }
            Expression::Size(size) => {
                amount = self.read(self.memory_pointer, size.or(Size::Byte));

                tracing::trace!(amount, ?size, "Moving left by current cell value...");
            }
            Expression::String(_) => unreachable!(),
        };

        self.memory_pointer = self.memory_pointer.saturating_sub(amount as usize);
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_right")]
    fn run_right(&mut self, expr: Expression) {
        let amount: u64;

        match expr {
            Expression::None => {
                amount = 1;

                tracing::trace!("Moving right by 1...");
            }
            Expression::Fixed(famount) => {
                amount = famount;

                tracing::trace!(amount, "Moving right by fixed value...");
            }
            Expression::Code(code, size) => {
                amount = self.run_expression(code, size);

                tracing::trace!(amount, ?size, "Moving right by computed value...");
            }
            Expression::Function(name) => {
                amount = self.run_function_call(None, name);

                tracing::trace!(amount, "Moving right by returned value...");
            }
            Expression::Size(size) => {
                amount = self.read(self.memory_pointer, size.or(Size::Byte));

                tracing::trace!(amount, ?size, "Moving right by current cell value...");
            }
            Expression::String(_) => unreachable!(),
        };

        self.memory_pointer = self.memory_pointer.saturating_add(amount as usize);
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_goto")]
    fn run_goto(&mut self, expr: Expression) {
        let amount: u64;

        match expr {
            Expression::None => {
                amount = self.read(self.memory_pointer, Size::Byte);

                tracing::trace!(amount, "Going to cell at current cell pointer...");
            }
            Expression::Size(size) => {
                amount = self.read(self.memory_pointer, size.or(Size::Byte));

                tracing::trace!(amount, ?size, "Going to cell at current cell pointer...");
            }
            Expression::Fixed(famount) => {
                amount = famount;

                tracing::trace!(amount, "Going to cell at fixed pointer...");
            }
            Expression::Code(code, size) => {
                amount = self.run_expression(code, size);

                tracing::trace!(amount, "Going to cell at computed pointer...");
            }
            Expression::Function(name) => {
                amount = self.run_function_call(None, name);

                tracing::trace!(amount, "Going to cell at returned value...");
            }
            Expression::String(_) => unreachable!(),
        };

        self.memory_pointer = amount as usize;
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_input")]
    fn run_input(&mut self, target: Option<Expression>) {
        let mut buffer = [0u8];

        let target = self.run_target(target);

        let mut stdin = io::stdin();
        if stdin.read(&mut buffer).unwrap() == 1 {
            self.write(target, buffer[0] as u64, Size::Byte);
            tracing::trace!(
                byte = buffer[0],
                "Executed , command. Wrote byte to memory."
            );
        } else {
            tracing::trace!("Executed , command. Nothing was written to memory.");
        }
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_output")]
    fn run_output(&mut self, size: Size, value: Expression) {
        match value {
            Expression::None => {
                let value = self.read(self.memory_pointer, size.or(Size::Byte));

                io::stdout()
                    .write_all(&size.or(Size::Byte).to_be_bytes(value))
                    .unwrap();
                self.last_output = Some(value as u8 as char);

                tracing::trace!(
                    output = value,
                    "Executed . command. Writing current value to output."
                );
            }
            Expression::Code(code, rsize) => {
                let result = self.run_expression(code, rsize);
                io::stdout()
                    .write_all(&size.or(rsize.or(Size::Byte)).to_be_bytes(result))
                    .unwrap();
                self.last_output = Some(result as u8 as char);

                tracing::trace!(
                    output = result,
                    "Executed . command. Writing computed value to output."
                );
            }
            Expression::Fixed(value) => {
                io::stdout()
                    .write_all(&size.or(Size::Byte).to_be_bytes(value))
                    .unwrap();
                self.last_output = Some(value as u8 as char);

                tracing::trace!(
                    output = value,
                    "Executed . command. Writing fixed value to output."
                );
            }
            Expression::String(contents) => {
                io::stdout().write_all(contents.as_bytes()).unwrap();
                if !contents.is_empty() {
                    self.last_output = Some(*contents.chars().collect::<Vec<_>>().last().unwrap());
                }

                tracing::trace!(
                    output = contents,
                    "Executed . command. Writing fixed string to output."
                );
            }
            Expression::Function(name) => {
                let result = self.run_function_call(None, name);
                io::stdout()
                    .write_all(&Size::Byte.to_be_bytes(result))
                    .unwrap();
                self.last_output = Some(result as u8 as char);

                tracing::trace!(
                    output = result,
                    "Executed . command. Writing returned value to output."
                );
            }
            Expression::Size(_) => unreachable!(),
        }

        io::stdout().flush().unwrap();
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_debug_output")]
    fn run_debug_output(&mut self, size: Size, value: Expression) {
        match value {
            Expression::None => {
                let value = self.read(self.memory_pointer, size.or(Size::Byte));

                print!(
                    "{}",
                    size.or(Size::Byte)
                        .to_be_bytes(value)
                        .iter()
                        .map(|b| format!("{b:0>3}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                io::stdout().flush().unwrap();

                // Works well enough since checks only check for new lines.
                self.last_output = Some('0');

                tracing::trace!(
                    output = value,
                    "Executed * command. Writing current value to output."
                );
            }
            Expression::Code(code, rsize) => {
                let value = self.run_expression(code, rsize);

                print!(
                    "{}",
                    size.or(Size::Byte)
                        .to_be_bytes(value)
                        .iter()
                        .map(|b| format!("{b:0>3}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                io::stdout().flush().unwrap();

                // Works well enough since checks only check for new lines.
                self.last_output = Some('0');

                tracing::trace!(
                    output = value,
                    "Executed * command. Writing computed value to output."
                );
            }
            Expression::Fixed(value) => {
                print!(
                    "{}",
                    size.or(Size::Byte)
                        .to_be_bytes(value)
                        .iter()
                        .map(|b| format!("{b:0>3}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                io::stdout().flush().unwrap();

                // Works well enough since checks only check for new lines.
                self.last_output = Some('0');

                tracing::trace!(
                    output = value,
                    "Executed * command. Writing fixed value to output."
                );
            }
            Expression::Function(name) => {
                let value = self.run_function_call(None, name);

                print!(
                    "{}",
                    size.or(Size::Byte)
                        .to_be_bytes(value)
                        .iter()
                        .map(|b| format!("{b:0>3}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                io::stdout().flush().unwrap();

                // Works well enough since checks only check for new lines.
                self.last_output = Some('0');

                tracing::trace!(
                    output = value,
                    "Executed * command. Writing returned value to output."
                );
            }
            Expression::String(_) => unreachable!(),
            Expression::Size(_) => unreachable!(),
        }

        io::stdout().flush().unwrap();
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_pointer")]
    fn run_pointer(&mut self, target: Option<Expression>, size: Size) {
        // size will only be different from `expr.size()` when `!expr.is_code()`.

        let target = self.run_target(target);

        self.write(target, self.memory_pointer as u64, size.or(Size::Byte));
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_while")]
    fn run_while(
        &mut self,
        code: Vec<Statement>,
        expr: Expression,
        is_negated: bool,
    ) -> Option<u64> {
        loop {
            let cell = match &expr {
                Expression::None => self.read(self.memory_pointer, Size::Byte),
                Expression::Code(code, size) => self.run_expression(code.clone(), *size),
                Expression::Size(size) => self.read(self.memory_pointer, size.or(Size::Byte)),
                Expression::Function(name) => self.run_function_call(None, name.clone()),
                Expression::Fixed(_) => unreachable!(),
                Expression::String(_) => unreachable!(),
            };

            if (!is_negated && cell == 0) || (is_negated && cell != 0) {
                break;
            }

            let snapshot = self.get_scope_snapshot();
            let result = self.run_statements(code.clone(), Size::None);
            self.set_scope_snapshot(snapshot);

            if result.is_some() {
                return result;
            }
        }

        None
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_repeat")]
    fn run_repeat(&mut self, code: Vec<Statement>, expr: Expression) -> Option<u64> {
        let iterations = match expr {
            Expression::Fixed(amount) => amount,
            Expression::Code(code, size) => self.run_expression(code, size),
            Expression::Size(size) => self.read(self.memory_pointer, size.or(Size::Byte)),
            Expression::None => self.read(self.memory_pointer, Size::Byte),
            Expression::Function(name) => self.run_function_call(None, name),
            Expression::String(_) => unreachable!(),
        };

        for _ in 0..iterations {
            let snapshot = self.get_scope_snapshot();
            let result = self.run_statements(code.clone(), Size::None);
            self.set_scope_snapshot(snapshot);

            if result.is_some() {
                return result;
            }
        }

        None
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_function_def")]
    fn run_function_def(&mut self, name: String, code: Vec<Statement>, size: Size) {
        self.functions.insert(name, Function { code, size });
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_function_call")]
    fn run_function_call(&mut self, target: Option<Expression>, name: String) -> u64 {
        let target_is_some = target.is_some();

        let target = self.run_target(target);
        let function = self.functions.get(&name).unwrap();
        let function_size = function.size;

        let init_cursor = self.memory_pointer;
        self.memory_pointer = target;

        let snapshot = self.get_scope_snapshot();

        let result = self.run_statements(function.code.clone(), function_size);
        let result = if let Some(result) = result {
            result
        } else {
            self.read(self.memory_pointer, function_size.or(Size::Byte))
        };

        self.set_scope_snapshot(snapshot);

        if target_is_some {
            self.memory_pointer = init_cursor;
        }

        result
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_return")]
    fn run_return(&mut self, expr: Expression, size: Size) -> u64 {
        match expr {
            Expression::Code(code, size) => {
                size.or(Size::Byte).wrap(self.run_expression(code, size))
            }
            Expression::Fixed(amount) => size.or(Size::Byte).wrap(amount),
            Expression::None => self.read(self.memory_pointer, size.or(Size::Byte)),
            Expression::Function(name) => self.run_function_call(None, name),
            Expression::String(_) => unreachable!(),
            Expression::Size(_) => unreachable!(),
        }
    }

    #[instrument(skip_all, target = "hf::language::runtime::Runtime::run_flag_carry")]
    fn run_flag_carry(&mut self, target: Option<Expression>) {
        let target = self.run_target(target);

        self.write(target, if self.flag_carry { 1 } else { 0 }, Size::Byte);
    }

    #[instrument(skip_all)]
    fn run_if(&mut self, branches: Vec<IfBranch>) {
        for branch in branches {
            match branch {
                IfBranch::If {
                    expr,
                    is_negated,
                    code,
                } => {
                    let cell = match &expr {
                        Expression::None => self.read(self.memory_pointer, Size::Byte),
                        Expression::Code(code, size) => self.run_expression(code.clone(), *size),
                        Expression::Size(size) => {
                            self.read(self.memory_pointer, size.or(Size::Byte))
                        }
                        Expression::Function(name) => self.run_function_call(None, name.clone()),
                        Expression::Fixed(_) => unreachable!(),
                        Expression::String(_) => unreachable!(),
                    };

                    if (!is_negated && cell == 0) || (is_negated && cell != 0) {
                        continue;
                    }

                    let snapshot = self.get_scope_snapshot();
                    self.run_statements(code, Size::None);
                    self.set_scope_snapshot(snapshot);
                    break;
                }
                IfBranch::Else { code } => {
                    let snapshot = self.get_scope_snapshot();
                    self.run_statements(code, Size::None);
                    self.set_scope_snapshot(snapshot);
                    break;
                }
            }
        }
    }
}

struct FullSnapshot {
    pub memory: Vec<u8>,
    pub memory_pointer: usize,
    pub functions: HashMap<String, Function>,
    pub flag_carry: bool,
}

struct ScopeSnapshot {
    pub functions: HashMap<String, Function>,
    pub flag_carry: bool,
}

#[derive(Clone)]
pub struct Function {
    code: Vec<Statement>,
    size: Size,
}
