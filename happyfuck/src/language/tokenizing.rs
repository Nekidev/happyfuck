use tracing::instrument;

use crate::language::errors::SyntaxError;

const NOTHING: [char; 4] = [' ', '\t', '\n', '\r'];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// +
    Plus,

    /// -
    Minus,

    /// <
    Left,

    /// >
    Right,

    /// =
    Set,

    /// ~
    Goto,

    /// ,
    Input,

    /// .
    Output,

    /// $
    Pointer,

    /// {
    BraceLeft,

    /// }
    BraceRight,

    /// [
    BracketLeft,

    /// ]
    BracketRight,

    /// (
    ParenthesisLeft,

    /// )
    ParenthesisRight,

    /// #
    Comment(String),

    /// "hey!"
    String(String),

    /// 123
    Number(String),

    /// Whitespaces, new lines, tabs, and returns.
    Nothing(String),

    /// b, w, d, q
    Size(char),

    /// @
    Target,

    /// &
    FunctionDefinition(String),

    /// :
    FunctionBodyStart,

    /// ;
    FunctionBodyFinish,

    /// %
    FunctionCallStatement(String),

    /// ?
    FunctionCallExpression(String),

    /// C
    FlagCarry,

    /// R
    Return,

    /// *
    DebugOutput,

    /// !
    Negation,

    /// I
    If,

    /// L
    ElseIf,

    /// E
    Else,

    /// F
    IfEnd,
}

impl Token {
    pub fn is_meaningful(&self) -> bool {
        !matches!(self, Token::Comment(_) | Token::Nothing(_))
    }
}

#[derive(Default)]
pub struct Tokenizer {
    cursor: usize,
    code: Vec<char>,
}

impl Tokenizer {
    fn read(&self) -> Option<char> {
        self.code.get(self.cursor).cloned()
    }

    fn next(&mut self) {
        self.cursor += 1;
    }

    #[instrument(skip_all)]
    pub fn tokenize(&mut self, code: &str) -> Result<Vec<Token>, SyntaxError> {
        let mut tokens = vec![];

        self.code.append(&mut code.chars().collect());

        while let Some(ch) = self.read() {
            let token = match ch {
                '+' => self.tokenize_plus(),
                '-' => self.tokenize_minus(),
                '<' => self.tokenize_left(),
                '>' => self.tokenize_right(),
                '=' => self.tokenize_set(),
                '~' => self.tokenize_goto(),
                ',' => self.tokenize_input(),
                '.' => self.tokenize_output(),
                '@' => self.tokenize_target(),
                '$' => self.tokenize_pointer(),
                '{' => self.tokenize_brace_left(),
                '}' => self.tokenize_brace_right(),
                '[' => self.tokenize_bracket_left(),
                ']' => self.tokenize_bracket_right(),
                '(' => self.tokenize_parenthesis_left(),
                ')' => self.tokenize_parenthesis_right(),
                '#' => self.tokenize_comment(),
                '&' => self.tokenize_function_def()?,
                ':' => self.tokenize_function_body_start(),
                ';' => self.tokenize_function_body_finish(),
                '%' => self.tokenize_function_call_statement()?,
                '?' => self.tokenize_function_call_expression()?,
                'C' => self.tokenize_flag_carry(),
                'R' => self.tokenize_return(),
                '*' => self.tokenize_debug_output(),
                '!' => self.tokenize_negation(),
                'I' => self.tokenize_if(),
                'L' => self.tokenize_else_if(),
                'E' => self.tokenize_else(),
                'F' => self.tokenize_if_end(),
                '\'' | '"' => self.tokenize_string()?,
                '0'..='9' => self.tokenize_number(),
                'b' | 'w' | 'd' | 'q' => self.tokenize_size(),
                '\n' | ' ' | '\t' | '\r' => self.tokenize_nothing(),
                _ => {
                    for _ in 0..code.len() {
                        self.code.pop();
                    }

                    return Err(SyntaxError::new(
                        format!("An unexpected token was found: {ch}"),
                        self.cursor..(self.cursor + 1),
                        true,
                    ));
                }
            };

            tokens.push(token);
        }

        Ok(tokens)
    }

    #[instrument(skip_all)]
    fn tokenize_plus(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized +");
        Token::Plus
    }

    #[instrument(skip_all)]
    fn tokenize_minus(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized -");
        Token::Minus
    }

    #[instrument(skip_all)]
    fn tokenize_left(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized <");
        Token::Left
    }

    #[instrument(skip_all)]
    fn tokenize_right(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized >");
        Token::Right
    }

    #[instrument(skip_all)]
    fn tokenize_set(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized =");
        Token::Set
    }

    #[instrument(skip_all)]
    fn tokenize_goto(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized ~");
        Token::Goto
    }

    #[instrument(skip_all)]
    fn tokenize_input(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized ,");
        Token::Input
    }

    #[instrument(skip_all)]
    fn tokenize_output(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized .");
        Token::Output
    }

    #[instrument(skip_all)]
    fn tokenize_target(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized @");
        Token::Target
    }

    #[instrument(skip_all)]
    fn tokenize_pointer(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized $");
        Token::Pointer
    }

    #[instrument(skip_all)]
    fn tokenize_brace_left(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized {{");
        Token::BraceLeft
    }

    #[instrument(skip_all)]
    fn tokenize_brace_right(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized }}");
        Token::BraceRight
    }

    #[instrument(skip_all)]
    fn tokenize_bracket_left(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized [");
        Token::BracketLeft
    }

    #[instrument(skip_all)]
    fn tokenize_bracket_right(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized ]");
        Token::BracketRight
    }

    #[instrument(skip_all)]
    fn tokenize_parenthesis_left(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized (");
        Token::ParenthesisLeft
    }

    #[instrument(skip_all)]
    fn tokenize_parenthesis_right(&mut self) -> Token {
        self.next();
        tracing::trace!("Tokenized )");
        Token::ParenthesisRight
    }

    #[instrument(skip_all)]
    fn tokenize_string(&mut self) -> Result<Token, SyntaxError> {
        let init_cursor = self.cursor;

        let quote = self.read().unwrap();

        self.next();

        let mut contents = String::new();

        while let Some(ch) = self.read() {
            match ch {
                '\\' => {
                    self.next();
                    let Some(ch) = self.read() else {
                        return Err(SyntaxError::new(
                            format!("An opening quote ({quote}) without a closing one was found."),
                            init_cursor..self.cursor,
                            false,
                        ));
                    };

                    let ch = match ch {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '0' => '\0',
                        '"' => '"',
                        '\'' => '\'',
                        '\\' => '\\',
                        _ => {
                            return Err(SyntaxError::new(
                                "An escape backslash (\\) without a proper escape sequence was found.",
                                self.cursor..self.cursor + 1,
                                true,
                            ));
                        }
                    };

                    contents.push(ch);
                }
                '"' | '\'' if ch == quote => {
                    self.next();
                    tracing::trace!(contents, "Tokenized {quote} string");
                    return Ok(Token::String(contents));
                }
                _ => {
                    contents.push(ch);
                }
            }

            self.next();
        }

        Err(SyntaxError::new(
            format!("An opening quote ({quote}) without a closing one was found."),
            init_cursor..self.cursor,
            false,
        ))
    }

    #[instrument(skip_all)]
    fn tokenize_comment(&mut self) -> Token {
        self.next();

        let mut contents = String::new();

        while let Some(ch) = self.read() {
            if ch == '\n' {
                break;
            }

            contents.push(ch);
            self.next();
        }

        tracing::trace!(?contents, "Tokenized #");

        Token::Comment(contents)
    }

    #[instrument(skip_all)]
    fn tokenize_number(&mut self) -> Token {
        let mut contents = String::new();

        while let Some(ch) = self.read() {
            tracing::trace!(?ch, "Processing char for number...");

            if !ch.is_ascii_digit() {
                break;
            }

            contents.push(ch);
            self.next();
        }

        tracing::trace!(?contents, "Tokenized number");

        Token::Number(contents)
    }

    #[instrument(skip_all)]
    fn tokenize_nothing(&mut self) -> Token {
        self.next();
        let mut contents = String::new();

        while let Some(ch) = self.read() {
            if !NOTHING.contains(&ch) {
                break;
            }

            contents.push(ch);
            self.next();
        }

        tracing::trace!(?contents, "Tokenized nothing");
        Token::Nothing(contents)
    }

    #[instrument(skip_all)]
    fn tokenize_size(&mut self) -> Token {
        let ch = self.read().unwrap();
        self.next();

        tracing::trace!(?ch, "Tokenized size");

        Token::Size(ch)
    }

    #[instrument(skip_all)]
    fn tokenize_function_def(&mut self) -> Result<Token, SyntaxError> {
        self.next();

        let mut name = String::new();

        while let Some(ch) = self.read() {
            match ch {
                'a'..='z' | '_' => name.push(ch),
                _ => break,
            }

            self.next();
        }

        if name.is_empty() {
            Err(SyntaxError::new(
                "Function definitions require a name.",
                self.cursor..self.cursor + 1,
                true,
            ))
        } else {
            Ok(Token::FunctionDefinition(name))
        }
    }

    #[instrument(skip_all)]
    fn tokenize_function_body_start(&mut self) -> Token {
        self.next();
        Token::FunctionBodyStart
    }

    #[instrument(skip_all)]
    fn tokenize_function_body_finish(&mut self) -> Token {
        self.next();
        Token::FunctionBodyFinish
    }

    #[instrument(skip_all)]
    fn tokenize_function_call_statement(&mut self) -> Result<Token, SyntaxError> {
        self.next();

        let mut name = String::new();

        while let Some(ch) = self.read() {
            match ch {
                'a'..='z' | '_' => name.push(ch),
                _ => break,
            }

            self.next();
        }

        if name.is_empty() {
            Err(SyntaxError::new(
                "Function calls require a function name.",
                self.cursor..self.cursor + 1,
                true,
            ))
        } else {
            Ok(Token::FunctionCallStatement(name))
        }
    }

    #[instrument(skip_all)]
    fn tokenize_function_call_expression(&mut self) -> Result<Token, SyntaxError> {
        self.next();

        let mut name = String::new();

        while let Some(ch) = self.read() {
            match ch {
                'a'..='z' | '_' => name.push(ch),
                _ => break,
            }

            self.next();
        }

        if name.is_empty() {
            Err(SyntaxError::new(
                "Function calls require a function name.",
                self.cursor..self.cursor + 1,
                true,
            ))
        } else {
            Ok(Token::FunctionCallExpression(name))
        }
    }

    #[instrument(skip_all)]
    fn tokenize_flag_carry(&mut self) -> Token {
        self.next();
        Token::FlagCarry
    }

    #[instrument(skip_all)]
    fn tokenize_return(&mut self) -> Token {
        self.next();
        Token::Return
    }

    #[instrument(skip_all)]
    fn tokenize_debug_output(&mut self) -> Token {
        self.next();
        Token::DebugOutput
    }

    #[instrument(skip_all)]
    fn tokenize_negation(&mut self) -> Token {
        self.next();
        Token::Negation
    }

    #[instrument(skip_all)]
    fn tokenize_if(&mut self) -> Token {
        self.next();
        Token::If
    }

    #[instrument(skip_all)]
    fn tokenize_else_if(&mut self) -> Token {
        self.next();
        Token::ElseIf
    }

    #[instrument(skip_all)]
    fn tokenize_else(&mut self) -> Token {
        self.next();
        Token::Else
    }

    #[instrument(skip_all)]
    fn tokenize_if_end(&mut self) -> Token {
        self.next();
        Token::IfEnd
    }
}
