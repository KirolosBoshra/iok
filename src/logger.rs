use crate::lexer::Loc;
use std::fmt;

#[derive(Debug)]
pub enum ErrorType {
    Lexing,
    Parsing,
    RunTime,
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Logger;

impl Logger {
    pub fn error(msg: &str, loc: Option<Loc>, err: ErrorType) {
        if let Some(l) = loc {
            eprintln!("{err} Error:\n\t{msg} at line {}:{}", l.y, l.x);
        } else {
            eprintln!("{err} Error:\n\t{msg}")
        }
    }
}
