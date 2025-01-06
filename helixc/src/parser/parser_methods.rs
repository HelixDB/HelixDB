use super::helix_parser::Rule;
use core::fmt;

pub trait Parser {
    fn parse(&self, input: &str) -> Result<(), String>;
}

#[derive(Debug)]
pub enum ParserError {
    ParseError(String),
    LexError(String),
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParserError::ParseError(e) => write!(f, "Parse error: {}", e),
            ParserError::LexError(e) => write!(f, "Lex error: {}", e),
        }
    }
}

impl From<pest::error::Error<Rule>> for ParserError {
    fn from(e: pest::error::Error<Rule>) -> Self {
        ParserError::ParseError(e.to_string())
    }
}

impl From<String> for ParserError {
    fn from(e: String) -> Self {
        ParserError::LexError(e)
    }
}

impl From<&'static str> for ParserError {
    fn from(e: &'static str) -> Self {
        ParserError::LexError(e.to_string())
    }
}
