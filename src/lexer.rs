use std::{iter::Peekable, str::Chars};

use crate::logger::{ErrorType, Logger};
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    static ref KEYWORDS: HashMap<&'static str, TokenType> = {
        let mut map = HashMap::new();
        map.insert("exit", TokenType::Exit);
        map.insert("let", TokenType::Let);
        map.insert("if", TokenType::If);
        map.insert("els", TokenType::Els);
        map.insert("elsif", TokenType::ElsIf);
        map.insert("while", TokenType::While);
        map.insert("for", TokenType::For);
        map.insert("fn", TokenType::Fn);
        map.insert("struct", TokenType::Struct);
        map.insert("ret", TokenType::Ret);
        map.insert("true", TokenType::Bool(true));
        map.insert("false", TokenType::Bool(false));
        map.insert("null", TokenType::Null);
        map.insert("write", TokenType::Write);
        map.insert("import", TokenType::Import);
        map
    };
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Number(f64),
    Bool(bool),
    String(String),
    Null,
    Plus,
    PlusEqu,
    DPlus,
    Minus,
    DMinus,
    Multiply,
    Divide,
    Equal,
    EquEqu,
    Bang,
    NotEqu,
    Greater,
    Less,
    GreatEqu,
    LessEqu,
    BitAnd,
    And,
    BitOR,
    Or,
    Shl,
    Shr,
    OpenParen,
    CloseParen,
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Comma,
    Semi,
    Colon,
    DColon,
    Dot,
    DDot,
    ThinArrow,
    FatArrow,
    Let,
    Exit,
    Ident(String),
    If,
    Els,
    ElsIf,
    While,
    For,
    Fn,
    Ret,
    Struct,
    Write,
    Import,
    As,
}

#[derive(Debug, Clone, Copy)]
pub struct Loc {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token: TokenType,
    pub loc: Loc,
}

#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    curr_loc: Loc,
    pub iter: Peekable<Chars<'a>>,
}
impl<'a> Lexer<'a> {
    pub fn new(input: &'a String) -> Lexer<'a> {
        let iter = input.chars().peekable();
        Lexer {
            curr_loc: Loc { x: 1, y: 1 },
            iter,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();

        while let Some(&c) = self.iter.peek() {
            match c {
                'a'..='z' | '_' | 'A'..='Z' => {
                    let mut buf = String::new();
                    while let Some(&c) = self.iter.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            buf.push(c);
                            self.next();
                        } else {
                            break;
                        }
                    }
                    if let Some(token_type) = KEYWORDS.get(buf.as_str()) {
                        tokens.push(Token {
                            token: token_type.clone(),
                            loc: self.curr_loc,
                        });
                    } else {
                        tokens.push(Token {
                            token: TokenType::Ident(buf),
                            loc: self.curr_loc,
                        });
                    }
                }
                '0'..='9' => {
                    let mut number = String::new();
                    while let Some(&c) = self.iter.peek() {
                        if c.is_digit(10) || (c == '.' && self.iter.clone().nth(1) != Some('.')) {
                            number.push(c);
                            self.next();
                        } else {
                            break;
                        }
                    }
                    let num = number.parse().unwrap();
                    tokens.push(Token {
                        token: TokenType::Number(num),
                        loc: self.curr_loc,
                    });
                }
                '\"' => {
                    self.next();
                    let mut string = String::new();
                    while let Some(&c) = self.iter.peek() {
                        match c {
                            '\"' => {
                                self.next();
                                break;
                            }
                            '\\' => {
                                string.push(c);
                                self.next();
                                if *self.iter.peek().unwrap() == '\"' {
                                    string.push('\"');
                                    self.next();
                                }
                            }
                            _ => {
                                string.push(c);
                                self.next();
                            }
                        }
                    }
                    tokens.push(Token {
                        token: TokenType::String(string),
                        loc: self.curr_loc,
                    });
                }
                '(' => {
                    tokens.push(Token {
                        token: TokenType::OpenParen,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                ')' => {
                    tokens.push(Token {
                        token: TokenType::CloseParen,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                '[' => {
                    tokens.push(Token {
                        token: TokenType::OpenSquare,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                ']' => {
                    tokens.push(Token {
                        token: TokenType::CloseSquare,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                '{' => {
                    tokens.push(Token {
                        token: TokenType::OpenCurly,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                '}' => {
                    tokens.push(Token {
                        token: TokenType::CloseCurly,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                '+' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        match c {
                            '+' => {
                                tokens.push(Token {
                                    token: TokenType::DPlus,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            '=' => {
                                tokens.push(Token {
                                    token: TokenType::PlusEqu,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            _ => {
                                tokens.push(Token {
                                    token: TokenType::Plus,
                                    loc: self.curr_loc,
                                });
                            }
                        }
                    }
                }
                '-' => {
                    self.next();
                    match *self.iter.peek().unwrap() {
                        '-' => {
                            self.next();
                            tokens.push(Token {
                                token: TokenType::DMinus,
                                loc: self.curr_loc,
                            });
                        }
                        '>' => {
                            self.next();
                            tokens.push(Token {
                                token: TokenType::ThinArrow,
                                loc: self.curr_loc,
                            });
                        }
                        _ => tokens.push(Token {
                            token: TokenType::Minus,
                            loc: self.curr_loc,
                        }),
                    }
                }
                '*' => {
                    tokens.push(Token {
                        token: TokenType::Multiply,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                '/' => {
                    self.next();
                    if *self.iter.peek().unwrap() == '/' {
                        while *self.iter.peek().unwrap() != '\n' {
                            self.next();
                        }
                    } else {
                        tokens.push(Token {
                            token: TokenType::Divide,
                            loc: self.curr_loc,
                        });
                    }
                }
                '=' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        match c {
                            '=' => {
                                tokens.push(Token {
                                    token: TokenType::EquEqu,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            '>' => {
                                tokens.push(Token {
                                    token: TokenType::FatArrow,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            _ => {
                                tokens.push(Token {
                                    token: TokenType::Equal,
                                    loc: self.curr_loc,
                                });
                            }
                        }
                    }
                }
                '!' => {
                    self.next();
                    if *self.iter.peek().unwrap_or(&' ') == '=' {
                        tokens.push(Token {
                            token: TokenType::NotEqu,
                            loc: self.curr_loc,
                        });
                        self.next();
                    } else {
                        tokens.push(Token {
                            token: TokenType::Bang,
                            loc: self.curr_loc,
                        });
                    }
                }
                '>' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        match c {
                            '=' => {
                                tokens.push(Token {
                                    token: TokenType::GreatEqu,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            '>' => {
                                tokens.push(Token {
                                    token: TokenType::Shr,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            _ => {
                                tokens.push(Token {
                                    token: TokenType::Greater,
                                    loc: self.curr_loc,
                                });
                            }
                        }
                    }
                }

                '<' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        match c {
                            '=' => {
                                tokens.push(Token {
                                    token: TokenType::LessEqu,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            '<' => {
                                tokens.push(Token {
                                    token: TokenType::Shl,
                                    loc: self.curr_loc,
                                });
                                self.next();
                            }
                            _ => {
                                tokens.push(Token {
                                    token: TokenType::Less,
                                    loc: self.curr_loc,
                                });
                            }
                        }
                    }
                }
                '&' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        if *c == '&' {
                            tokens.push(Token {
                                token: TokenType::And,
                                loc: self.curr_loc,
                            });
                            self.next();
                        } else {
                            tokens.push(Token {
                                token: TokenType::BitAnd,
                                loc: self.curr_loc,
                            })
                        }
                    }
                }
                '|' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        if *c == '|' {
                            tokens.push(Token {
                                token: TokenType::Or,
                                loc: self.curr_loc,
                            });
                            self.next();
                        } else {
                            tokens.push(Token {
                                token: TokenType::BitOR,
                                loc: self.curr_loc,
                            });
                        }
                    }
                }
                '.' => {
                    self.next();
                    if *self.iter.peek().unwrap_or(&' ') == '.' {
                        tokens.push(Token {
                            token: TokenType::DDot,
                            loc: self.curr_loc,
                        });
                        self.next();
                    } else {
                        tokens.push(Token {
                            token: TokenType::Dot,
                            loc: self.curr_loc,
                        });
                    }
                }
                ',' => {
                    self.next();
                    tokens.push(Token {
                        token: TokenType::Comma,
                        loc: self.curr_loc,
                    });
                }
                ';' => {
                    tokens.push(Token {
                        token: TokenType::Semi,
                        loc: self.curr_loc,
                    });
                    self.next();
                }
                ':' => {
                    self.next();
                    if let Some(c) = self.iter.peek() {
                        if *c == ':' {
                            tokens.push(Token {
                                token: TokenType::DColon,
                                loc: self.curr_loc,
                            });
                            self.next();
                        } else {
                            tokens.push(Token {
                                token: TokenType::Colon,
                                loc: self.curr_loc,
                            })
                        }
                    }
                }

                '@' => {
                    tokens.push(Token {
                        token: TokenType::As,
                        loc: self.curr_loc,
                    });
                    self.next();
                }

                '\n' => {
                    self.curr_loc.x = 1;
                    self.curr_loc.y += 1;
                    self.iter.next();
                }
                ' ' | '\t' | '\r' => {
                    self.next();
                }
                _ => {
                    Logger::error(
                        &format!("Unexpected Token: {c}"),
                        self.curr_loc,
                        ErrorType::Lexing,
                    );
                    self.next();
                }
            }
        }

        tokens
    }
    fn next(&mut self) {
        self.curr_loc.x += 1;
        self.iter.next();
    }
}
