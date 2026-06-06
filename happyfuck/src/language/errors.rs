use std::range::Range;

#[derive(Debug, thiserror::Error)]
#[error("Syntax error at {position:?}: {message}")]
pub struct SyntaxError {
    pub message: String,
    pub position: Range<usize>,
    pub is_fatal: bool,
}

impl SyntaxError {
    pub fn new(
        message: impl Into<String>,
        position: impl Into<Range<usize>>,
        is_fatal: bool,
    ) -> Self {
        SyntaxError {
            message: message.into(),
            position: position.into(),
            is_fatal,
        }
    }
}
