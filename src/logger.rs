use crate::lexer::Loc;
use std::fmt;

#[derive(Debug)]
pub enum ErrorType {
    Lexing,
    Parsing,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Logger;

impl Logger {
    pub fn error(msg: &str, loc: Loc, err: ErrorType) {
        eprintln!("{err} Error:\n\t{msg} at line {}:{}", loc.y, loc.x);
    }
}
