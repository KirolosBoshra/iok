use crate::lexer::Loc;
use lazy_static::lazy_static;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

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

struct SourceInfo {
    name: String,
    lines: Vec<String>,
}

lazy_static! {
    static ref SOURCES: Mutex<Vec<SourceInfo>> = Mutex::new(Vec::new());
    static ref ERRORS: AtomicUsize = AtomicUsize::new(0);
}

pub struct Logger;

impl Logger {
    pub fn push_source(name: &str, text: &str) {
        let lines = text.lines().map(|l| l.to_string()).collect();
        SOURCES.lock().unwrap().push(SourceInfo {
            name: name.to_string(),
            lines,
        });
    }

    pub fn pop_source() {
        SOURCES.lock().unwrap().pop();
    }

    pub fn error_count() -> usize {
        ERRORS.load(Ordering::Relaxed)
    }

    pub fn error(msg: &str, loc: Option<Loc>, err: ErrorType) {
        ERRORS.fetch_add(1, Ordering::Relaxed);
        let sources = SOURCES.lock().unwrap();
        let source = sources.last();

        match (loc, source) {
            (Some(l), Some(src)) => {
                eprintln!("{err} Error: {msg}");
                eprintln!("  --> {}:{}:{}", src.name, l.y, l.x);
                if l.y >= 1 && l.y <= src.lines.len() {
                    let line = &src.lines[l.y - 1];
                    let caret_col = l.x.min(line.chars().count() + 1);
                    eprintln!("  {:<3}| {line}", l.y);
                    eprintln!("     | {}{}", " ".repeat(caret_col.saturating_sub(1)), "^");
                }
            }
            (Some(l), None) => eprintln!("{err} Error: {msg}\n\tat line {}:{}", l.y, l.x),
            (None, Some(src)) => eprintln!("{err} Error: {msg}\n\tin {}", src.name),
            (None, None) => eprintln!("{err} Error: {msg}"),
        }
    }
}

