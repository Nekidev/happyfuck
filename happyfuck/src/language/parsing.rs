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
    While(Vec<Statement>, Expression),
    Repeat(Vec<Statement>, Expression),
}

#[derive(Debug, Clone)]
pub enum Expression {
    None,
    Size(Size),
    Code(Vec<Statement>, Size),
    Fixed(u64),
    String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionKind {
    None,
    Size,
    Code,
    Fixed,
    String,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nesting {
    Braces,
    Brackets,
    Parentheses,
}

#[derive(Default)]
pub struct Parser {
    cursor: usize,
    tokens: Vec<Token>,

    pub nesting: Vec<Nesting>,
}

impl Parser {
    fn read(&self) -> Option<Token> {
        self.tokens.get(self.cursor).cloned()
    }

    fn next(&mut self) {
        tracing::trace!(new_cursor = self.cursor + 1, "Moved parser cursor.");
        self.cursor += 1;
    }

    fn error<T>(&self, message: impl Into<String>, is_fatal: bool) -> Result<T, SyntaxError> {
        Err(SyntaxError::new(message, 0..1, is_fatal))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::feed")]
    pub fn feed(&mut self, tokens: Vec<Token>) -> Result<Vec<Statement>, SyntaxError> {
        let tokens: Vec<_> = tokens.into_iter().filter(Token::is_meaningful).collect();
        self.tokens.extend_from_slice(&tokens);
        self.nesting.clear();

        let init_cursor = self.cursor;

        let result = self.parse();

        if let Err(error) = &result {
            self.cursor = init_cursor;

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
                Token::Left => self.parse_left()?,
                Token::Right => self.parse_right()?,
                Token::Goto => self.parse_goto()?,
                Token::Input => self.parse_input(None),
                Token::Output => self.parse_output()?,
                Token::Target => self.parse_target()?,
                Token::Pointer => self.parse_pointer(None)?,
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
                Token::Comment(_) => {
                    self.next();
                    continue;
                }
                Token::Nothing(_) => {
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
                let size = self.parse_size();

                tracing::trace!(?code, ?size, "Parsed code expression.");

                Ok(Expression::Code(code, size))
            }
            Token::Size(_) if kinds.contains(&ExpressionKind::Size) => {
                let size = self.parse_size();

                tracing::trace!(?size, "Parsed empty size expression.");

                Ok(Expression::Size(size))
            }
            _ if kinds.contains(&ExpressionKind::None) => Ok(Expression::None),
            _ => result_missing,
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
            ExpressionKind::Fixed,
        ])?;

        Ok(Statement::Left(expression))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_right")]
    fn parse_right(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let expression = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
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
        ])?;

        Ok(Statement::Output { size, value })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_pointer")]
    fn parse_pointer(&mut self, target: Option<Expression>) -> Result<Statement, SyntaxError> {
        self.next();

        let size = self.parse_size();

        Ok(Statement::Pointer { target, size })
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_target")]
    fn parse_target(&mut self) -> Result<Statement, SyntaxError> {
        self.next();

        let target = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Fixed,
            ExpressionKind::Code,
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
            Token::Pointer => self.parse_pointer(Some(target)),
            Token::Input => Ok(self.parse_input(Some(target))),
            _ => self.error(
                "Target operand (@) didn't have a corresponding write command.",
                true,
            ),
        }
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_brackets")]
    fn parse_brackets(&mut self) -> Result<Statement, SyntaxError> {
        self.next();
        self.nesting.push(Nesting::Brackets);

        let code = self.parse()?;
        let expr = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Code,
        ])?;

        Ok(Statement::While(code, expr))
    }

    #[instrument(skip_all, target = "hf::language::parsing::Parser::parse_parentheses")]
    fn parse_parentheses(&mut self) -> Result<Statement, SyntaxError> {
        self.next();
        self.nesting.push(Nesting::Parentheses);

        let code = self.parse()?;
        let expr = self.parse_expression([
            ExpressionKind::None,
            ExpressionKind::Size,
            ExpressionKind::Code,
            ExpressionKind::Fixed,
        ])?;

        tracing::trace!(?code, ?expr, "Parsed parentheses.");

        Ok(Statement::Repeat(code, expr))
    }
}
