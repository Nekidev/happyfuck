use std::collections::HashSet;

use tracing::instrument;

use crate::language::errors::SyntaxError;
use crate::language::tokenizing::Token;

#[derive(Debug, Clone)]
pub enum Statement {
    Add {
        target: Option<Expression>,
        size: Size,
        value: Expression,
    },
    Subtract {
        target: Option<Expression>,
        size: Size,
        value: Expression,
    },
    Set {
        target: Option<Expression>,
        size: Size,
        value: Expression,
    },
    Pointer {
        target: Option<Expression>,
        size: Size,
    },
    FlagCarry {
        target: Option<Expression>,
    },
    Left(Expression),
    Right(Expression),
    Goto(Expression),
    Input {
        target: Option<Expression>,
    },
    Output {
        size: Size,
        value: Expression,
    },
    DebugOutput {
        size: Size,
        value: Expression,
    },
    While {
        code: Vec<Statement>,
        expr: Expression,
        is_negated: bool,
    },
    Repeat(Vec<Statement>, Expression),
    FunctionDefinition {
        name: String,
        code: Vec<Statement>,
        size: Size,
    },
    FunctionCall {
        target: Option<Expression>,
        name: String,
    },
    Return {
        value: Expression,
    },
    If(Vec<IfBranch>),
}

#[derive(Debug, Clone)]
pub enum IfBranch {
    If {
        expr: Expression,
        is_negated: bool,
        code: Vec<Statement>,
    },
    Else {
        code: Vec<Statement>,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    None,
    Size(Size),
    Code(Vec<Statement>, Size),
    Fixed(u64),
    String(String),
    Function(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionKind {
    None,
    Size,
    Code,
    Fixed,
    String,
    Function,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size {
    None = 0,
    Byte = 1,
    Word = 2,
    DWord = 4,
    QWord = 8,
}

impl Size {
    pub fn is_none(&self) -> bool {
        self == &Size::None
    }

    pub fn is_some(&self) -> bool {
        self != &Size::None
    }

    pub fn wrap(&self, value: u64) -> u64 {
        match self {
            Size::None => 0,
            Size::Byte => value % 2u64.pow(8),
            Size::Word => value % 2u64.pow(16),
            Size::DWord => value % 2u64.pow(32),
            Size::QWord => value,
        }
    }

    pub fn or(self, size: Size) -> Size {
        if self.is_none() { size } else { self }
    }

    pub fn to_be_bytes(self, value: u64) -> Vec<u8> {
        match self {
            Size::None => panic!(),
            Size::Byte => value.to_be_bytes()[7..8].to_vec(),
            Size::Word => value.to_be_bytes()[6..8].to_vec(),
            Size::DWord => value.to_be_bytes()[4..8].to_vec(),
            Size::QWord => value.to_be_bytes().to_vec(),
        }
    }

    pub fn fits(&self, value: u64) -> bool {
        match self {
            Size::None => panic!(),
            Size::Byte => value <= u8::MAX as u64 + 1,
            Size::Word => value <= u16::MAX as u64 + 1,
            Size::DWord => value <= u32::MAX as u64 + 1,
            Size::QWord => true,
        }
    }

    pub fn overflowing_add(&self, a: u64, b: u64) -> (u64, bool) {
        let (result, is_overflowed) = a.overflowing_add(b);

        if is_overflowed {
            return (result, is_overflowed);
        }

        (self.wrap(result), self.fits(result))
    }

    pub fn overflowing_sub(&self, a: u64, b: u64) -> (u64, bool) {
        let (result, is_overflowed) = a.overflowing_sub(b);

        if is_overflowed {
            return (result, is_overflowed);
        }

        (self.wrap(result), self.fits(result))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nesting {
    Braces,
    Brackets,
    Parentheses,
    FunctionBody,
    If,
    ElseIf,
    Else,
}

#[derive(Default)]
pub struct Parser {
    cursor: usize,
    tokens: Vec<Token>,

    pub nesting: Vec<Nesting>,
    pub functions: HashSet<String>,
}

impl Parser {
    fn read(&self) -> Option<Token> {
        self.tokens.get(self.cursor).cloned()
    }

    fn read_last(&self) -> Option<Token> {
        self.tokens.get(self.cursor.wrapping_sub(1)).cloned()
    }

    fn next(&mut self) {
        tracing::trace!(new_cursor = self.cursor + 1, "Moved parser cursor.");
        self.cursor += 1;
    }

    fn error<T>(&self, message: impl Into<String>, is_fatal: bool) -> Result<T, SyntaxError> {
        Err(SyntaxError::new(message, 0..1, is_fatal))
    }

    #[instrument(skip_all)]
    pub fn undo(&mut self) {
        tracing::trace!("Undoing...");

        self.cursor += 1;
        // self.cursor = self.cursor.wrapping_sub(1);

        while let Some(token) = self.read() {
            tracing::trace!(?token, "Undoing token...");

            match token {
                Token::BraceLeft | Token::BracketLeft | Token::ParenthesisLeft => {
                    self.nesting.pop();
                }
                Token::FunctionBodyStart => {
                    self.nesting.pop();

                    // Add another token pop, once for the function name.
                    self.tokens.pop();
                }
                Token::BraceRight => self.nesting.push(Nesting::Braces),
                Token::BracketRight => self.nesting.push(Nesting::Brackets),
                Token::ParenthesisRight => self.nesting.push(Nesting::Parentheses),
                Token::FunctionBodyFinish => self.nesting.push(Nesting::FunctionBody),
                _ => {}
            }

            self.tokens.pop();
            self.cursor = self.cursor.wrapping_sub(1);

            if self.nesting.is_empty() {
                break;
            }
        }
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::feed")]
    pub fn feed(&mut self, tokens: Vec<Token>) -> Result<Vec<Statement>, SyntaxError> {
        let tokens: Vec<_> = tokens.into_iter().filter(Token::is_meaningful).collect();
        self.tokens.extend_from_slice(&tokens);
        self.nesting.clear();

        let init_cursor = self.cursor;
        let init_functions = self.functions.clone();

        let result = self.parse();

        if let Err(error) = &result {
            self.cursor = init_cursor;
            self.functions = init_functions;

            if error.is_fatal {
                for _ in 0..tokens.len() {
                    self.tokens.pop();
                }
            }
        }

        result
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse")]
    fn parse(&mut self) -> Result<Vec<Statement>, SyntaxError> {
        let mut statements = vec![];

        while let Some(token) = self.read() {
            let statement = match token {
                Token::Plus => self.parse_plus(None)?,
                Token::Minus => self.parse_minus(None)?,
                Token::Set => self.parse_set(None)?,
                Token::FlagCarry => self.parse_flag_carry(None),
                Token::Left => self.parse_left()?,
                Token::Right => self.parse_right()?,
                Token::Goto => self.parse_goto()?,
                Token::Input => self.parse_input(None),
                Token::Output => self.parse_output()?,
                Token::DebugOutput => self.parse_debug_output()?,
                Token::Target => self.parse_target()?,
                Token::Pointer => self.parse_pointer(None),
                Token::FunctionDefinition(name) => self.parse_function_def(name)?,
                Token::FunctionBodyStart => {
                    return self.error(
                        "Found a function body start outside of a function declaration.",
                        true,
                    );
                }
                Token::FunctionBodyFinish => {
                    if self.nesting.ends_with(&[Nesting::FunctionBody]) {
                        self.nesting.pop();
                        self.next();
                        break;
                    } else {
                        return self.error(
                            "Found a function body finish (;) outside a function declaration.",
                            true,
                        );
                    }
                }
                Token::BraceLeft => {
                    return self.error(
                        "Found an opening brace ({) for an expression without a statement to modify.",
                        true,
                    );
                }
                Token::BraceRight => {
                    if self.nesting.ends_with(&[Nesting::Braces]) {
                        self.nesting.pop();
                        self.next();
                        break;
                    } else {
                        return self.error(
                            "Found a closing brace (}) without a matching opening one ({).",
                            true,
                        );
                    }
                }
                Token::BracketLeft => self.parse_brackets()?,
                Token::BracketRight => {
                    if self.nesting.ends_with(&[Nesting::Brackets]) {
                        self.nesting.pop();
                        self.next();
                        break;
                    } else {
                        return self.error(
                            "Found a closing bracket (]) without a matching opening one ([).",
                            true,
                        );
                    }
                }
                Token::ParenthesisLeft => self.parse_parentheses()?,
                Token::ParenthesisRight => {
                    if self.nesting.ends_with(&[Nesting::Parentheses]) {
                        self.nesting.pop();
                        self.next();
                        break;
                    } else {
                        return self.error(
                            "Found a closing parenthesis ()) without a matching opening one (().",
                            true,
                        );
                    }
                }
                Token::Comment(_) | Token::Nothing(_) => {
                    self.next();
                    continue;
                }
                Token::Number(_) => {
                    return self.error(
                        "Numbers can only appear when an expression is required.",
                        true,
                    );
                }
                Token::String(_) => {
                    return self.error(
                        "Strings can only appear when an expression is required.",
                        true,
                    );
                }
                Token::Size(_) => {
                    return self.error("Sizes can only appear when an operator supports it.", true);
                }
                Token::FunctionCallStatement(name) => self.parse_function_call(None, name)?,
                Token::FunctionCallExpression(_) => {
                    return self.error(
                        "Using functions as statements require the % command, not the ? command.",
                        true,
                    );
                }
                Token::Return => self.parse_return()?,
                Token::Negation => {
                    return self.error("You cannot negate a statement, only expressions.", true);
                }
                Token::If => self.parse_if()?,
                Token::ElseIf | Token::Else => {
                    if self.nesting.ends_with(&[Nesting::If])
                        | self.nesting.ends_with(&[Nesting::ElseIf])
                    {
                        self.nesting.pop();
                        self.next();
                        break;
                    } else {
                        return self.error(
                            "You need an IF statement to be able to have ELSE (E/L) statements.",
                            true,
                        );
                    }
                }
                Token::IfEnd => {
                    if self.nesting.ends_with(&[Nesting::If])
                        | self.nesting.ends_with(&[Nesting::ElseIf])
                        | self.nesting.ends_with(&[Nesting::Else])
                    {
                        self.nesting.pop();
                        self.next();
                        break;
                    } else {
                        return self.error(
                            "You need an IF statement to be able to have an IF END (F) statements.",
                            true,
                        );
                    }
                
                }
            };

            statements.push(statement);
        }

        Ok(statements)
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_size")]
    fn parse_size(&mut self) -> Size {
        let Some(token) = self.read() else {
            return Size::None;
        };

        let size = match token {
            Token::Size(size) => match size {
                'b' => Size::Byte,
                'w' => Size::Word,
                'd' => Size::DWord,
                'q' => Size::QWord,
                _ => unreachable!(),
            },
            _ => Size::None,
        };

        if size.is_some() {
            self.next();
        }

        size
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_expression")]
    fn parse_expression<const N: usize>(
        &mut self,
        kinds: [ExpressionKind; N],
    ) -> Result<Expression, SyntaxError> {
        tracing::trace!("Parsing expression...");

        let result_missing = self.error("An expression was expected. None was found.", true);

        let Some(token) = self.read() else {
            tracing::trace!("No expression was found.");

            if kinds.contains(&ExpressionKind::None) {
                return Ok(Expression::None);
            } else {
                return result_missing;
            }
        };

        match token {
            Token::Number(number) if kinds.contains(&ExpressionKind::Fixed) => {
                self.next();
                tracing::trace!(number, "Parsed fixed number expression.");

                Ok(Expression::Fixed(number.parse().unwrap()))
            }
            Token::String(content) if kinds.contains(&ExpressionKind::String) => {
                self.next();
                tracing::trace!(content, "Parsed string expression.");

                Ok(Expression::String(content))
            }
            Token::BraceLeft if kinds.contains(&ExpressionKind::Code) => {
                self.nesting.push(Nesting::Braces);
                self.next();

                let code = self.parse()?;

                if self.read_last() != Some(Token::BraceRight) {
                    return self.error(
                        "Opening expression brace ({) did not have a matching closing one (}).",
                        false,
                    );
                }

                let size = self.parse_size();

                tracing::trace!(?code, ?size, "Parsed code expression.");

                Ok(Expression::Code(code, size))
            }
            Token::Size(_) if kinds.contains(&ExpressionKind::Size) => {
                let size = self.parse_size();

                tracing::trace!(?size, "Parsed empty size expression.");

                Ok(Expression::Size(size))
            }
            Token::FunctionCallExpression(name) => {
                self.next();

                if !self.functions.contains(&name) {
                    return self.error(
                        format!("No function called `{name}` is available in this scope."),
                        true,
                    );
                }

                tracing::trace!(name, "Parsed function call expression.");

                Ok(Expression::Function(name))
            }
            _ if kinds.contains(&ExpressionKind::None) => Ok(Expression::None),
            _ => result_missing,
        }
    }

    #[instrument(skip_all)]
    fn parse_negation(&mut self) -> bool {
        if self.read() == Some(Token::Negation) {
            self.next();
            true
        } else {
            false
        }
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_plus")]
    fn parse_plus(&mut self, target: Option<Expression>) -> Result<Statement, SyntaxError> {
        self.next();

        let size = self.parse_size();
        let value = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        tracing::trace!(?value, "Parsed + token.");

        Ok(Statement::Add {
            target,
            size,
            value,
        })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_minus")]
    fn parse_minus(&mut self, target: Option<Expression>) -> Result<Statement, SyntaxError> {
        self.next();

        let size = self.parse_size();
        let value = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Subtract {
            target,
            size,
            value,
        })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_set")]
    fn parse_set(&mut self, target: Option<Expression>) -> Result<Statement, SyntaxError> {
        self.next();

        let size = self.parse_size();
        let value = self.parse_expression([
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::String,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Set {
            target,
            size,
            value,
        })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_left")]
    fn parse_left(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let expression = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Size,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Left(expression))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_right")]
    fn parse_right(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let expression = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Size,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Right(expression))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_goto")]
    fn parse_goto(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let expression = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Goto(expression))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_input")]
    fn parse_input(&mut self, target: Option<Expression>) -> Statement {
        self.next();
        Statement::Input { target }
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_output")]
    fn parse_output(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let size = self.parse_size();
        let value = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::String,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Output { size, value })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_debug_output")]
    fn parse_debug_output(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let size = self.parse_size();
        let value = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::DebugOutput { size, value })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_pointer")]
    fn parse_pointer(&mut self, target: Option<Expression>) -> Statement {
        self.next();

        let size = self.parse_size();

        Statement::Pointer { target, size }
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_flag_carry")]
    fn parse_flag_carry(&mut self, target: Option<Expression>) -> Statement {
        self.next();
        Statement::FlagCarry { target }
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_target")]
    fn parse_target(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let target = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        let Some(token) = self.read() else {
            return self.error(
                "Target operand (@) didn't have a corresponding write command.",
                true,
            );
        };

        match token {
            Token::Plus => self.parse_plus(Some(target)),
            Token::Minus => self.parse_minus(Some(target)),
            Token::Set => self.parse_set(Some(target)),
            Token::Pointer => Ok(self.parse_pointer(Some(target))),
            Token::Input => Ok(self.parse_input(Some(target))),
            Token::FlagCarry => Ok(self.parse_flag_carry(Some(target))),
            Token::FunctionCallExpression(name) => self.parse_function_call(Some(target), name),
            _ => self.error(
                "Target operand (@) didn't have a corresponding write command.",
                true,
            ),
        }
    }

    #[instrument(skip_all)]
    fn parse_brackets(&mut self) -> Result<Statement, SyntaxError> {
        self.next();
        self.nesting.push(Nesting::Brackets);

        let code = self.parse()?;

        if self.read_last() != Some(Token::BracketRight) {
            return self.error(
                "Opening bracket ([) did not have a matching closing one (]).",
                false,
            );
        }

        let is_negated = self.parse_negation();

        let expr = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Code,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::While {
            code,
            expr,
            is_negated,
        })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_parentheses")]
    fn parse_parentheses(&mut self) -> Result<Statement, SyntaxError> {
        self.next();
        self.nesting.push(Nesting::Parentheses);

        let code = self.parse()?;

        if self.read_last() != Some(Token::ParenthesisRight) {
            return self.error(
                "Opening parenthesis (() did not have a matching closing one ()).",
                false,
            );
        }

        let expr = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        tracing::trace!(?code, ?expr, "Parsed parentheses.");

        Ok(Statement::Repeat(code, expr))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_if")]
    fn parse_function_def(&mut self, name: String) -> Result<Statement, SyntaxError> {
        if self.functions.contains(&name) {
            return self.error(
                format!(
                    concat!(
                        "Function `{}` already exists. You cannot define a function with the ",
                        "same name twice. {:?}"
                    ),
                    name, self.functions
                ),
                true,
            );
        }

        self.next();

        if self.read() != Some(Token::FunctionBodyStart) {
            return self.error(
                "Function declarations require a function body start. None was found.",
                true,
            );
        }

        self.nesting.push(Nesting::FunctionBody);
        self.next();

        self.functions.insert(name.clone());
        let snapshot = self.functions.clone();

        let code = self.parse()?;

        if self.read_last() != Some(Token::FunctionBodyFinish) {
            return self.error("Function body (:) was not closed (;).", false);
        }

        let size = self.parse_size();

        self.functions = snapshot;

        Ok(Statement::FunctionDefinition { name, code, size })
    }

    #[instrument(
        skip_all,
        target = "hf::language::parsing::Parser::parse_function_call"
    )]
    fn parse_function_call(
        &mut self,
        target: Option<Expression>,
        name: String,
    ) -> Result<Statement, SyntaxError> {
        if !self.functions.contains(&name) {
            return self.error(
                format!("There's no function `{name}` in the current scope."),
                true,
            );
        }

        self.next();
        Ok(Statement::FunctionCall { target, name })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_return")]
    fn parse_return(&mut self) -> Result<Statement, SyntaxError> {
        if !self.nesting.contains(&Nesting::FunctionBody) {
            return self.error("You cannot return outside of a function body.", true);
        }

        self.next();

        let value = self.parse_expression([
            ExpressionKind::Code,
            ExpressionKind::Fixed,
            ExpressionKind::Function,
        ])?;

        Ok(Statement::Return { value })
    }

    #[instrument(skip_all)]
    fn parse_if(&mut self) -> Result<Statement, SyntaxError> {
        let mut branches = vec![];

        self.next();

        loop {
            match self.read_last() {
                Some(token @ Token::If | token @ Token::ElseIf) => {
                    if token == Token::If {
                        self.nesting.push(Nesting::If);
                    } else {
                        self.nesting.push(Nesting::ElseIf);
                    }

                    let is_negated = self.parse_negation();
                    let expression = self.parse_expression([
                        ExpressionKind::None,
                        ExpressionKind::Size,
                        ExpressionKind::Code,
                        ExpressionKind::Function,
                    ])?;
                    let code = self.parse()?;

                    branches.push(IfBranch::If {
                        expr: expression,
                        is_negated,
                        code,
                    });
                }
                Some(Token::Else) => {
                    self.nesting.push(Nesting::Else);

                    let code = self.parse()?;

                    branches.push(IfBranch::Else { code });
                    break;
                }
                _ => {
                    break;
                }
            }
        }

        if self.read_last() != Some(Token::IfEnd) {
            return self.error("IF (I) statement did not have an IF closing (F).", false);
        }

        Ok(Statement::If(branches))
    }
}
