use std::{iter::Peekable, str::Chars};

#[derive(Debug, Clone)]
pub enum TokenType {
    Number(usize),
    String(String),
    Plus,
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
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    Comma,
    Semi,
    Dot,
    DDot,
    ThinArrow,
    Let,
    Exit,
    Ident(String),
    If,
    Els,
    ElsIf,
    While,
    For,
    // Empty,
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
    iter: Peekable<Chars<'a>>,
}
impl<'a> Lexer<'a> {
    pub fn new(input: &'a String) -> Lexer<'a> {
        let iter = input.chars().peekable();
        Lexer {
            curr_loc: Loc { x: 1, y: 0 },
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
                    match buf.as_str() {
                        "exit" => tokens.push(Token {
                            token: TokenType::Exit,
                            loc: self.curr_loc,
                        }),
                        "let" => tokens.push(Token {
                            token: TokenType::Let,
                            loc: self.curr_loc,
                        }),
                        "if" => tokens.push(Token {
                            token: TokenType::If,
                            loc: self.curr_loc,
                        }),
                        "els" => tokens.push(Token {
                            token: TokenType::Els,
                            loc: self.curr_loc,
                        }),
                        "elsif" => tokens.push(Token {
                            token: TokenType::ElsIf,
                            loc: self.curr_loc,
                        }),
                        "while" => tokens.push(Token {
                            token: TokenType::While,
                            loc: self.curr_loc,
                        }),
                        "for" => tokens.push(Token {
                            token: TokenType::For,
                            loc: self.curr_loc,
                        }),
                        _ => tokens.push(Token {
                            token: TokenType::Ident(buf),
                            loc: self.curr_loc,
                        }),
                    }
                }
                '0'..='9' => {
                    let mut number = String::new();
                    while let Some(&c) = self.iter.peek() {
                        if c.is_digit(10) {
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
                    if *self.iter.peek().unwrap() == '+' {
                        self.next();
                        tokens.push(Token {
                            token: TokenType::DPlus,
                            loc: self.curr_loc,
                        });
                    } else {
                        tokens.push(Token {
                            token: TokenType::Plus,
                            loc: self.curr_loc,
                        });
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
                    if *self.iter.peek().unwrap() == '=' {
                        tokens.push(Token {
                            token: TokenType::EquEqu,
                            loc: self.curr_loc,
                        });
                        self.next();
                    } else {
                        tokens.push(Token {
                            token: TokenType::Equal,
                            loc: self.curr_loc,
                        });
                    }
                }
                '!' => {
                    self.next();
                    if *self.iter.peek().unwrap() == '=' {
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
                    if *self.iter.peek().unwrap() == '=' {
                        tokens.push(Token {
                            token: TokenType::GreatEqu,
                            loc: self.curr_loc,
                        });
                        self.next();
                    } else {
                        tokens.push(Token {
                            token: TokenType::Greater,
                            loc: self.curr_loc,
                        });
                    }
                }

                '<' => {
                    self.next();
                    if *self.iter.peek().unwrap() == '=' {
                        tokens.push(Token {
                            token: TokenType::LessEqu,
                            loc: self.curr_loc,
                        });
                        self.next();
                    } else {
                        tokens.push(Token {
                            token: TokenType::Less,
                            loc: self.curr_loc,
                        });
                    }
                }
                '.' => {
                    self.next();
                    if *self.iter.peek().unwrap() == '.' {
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
                '\n' => {
                    self.curr_loc.x = 1;
                    self.curr_loc.y += 1;
                    self.iter.next();
                }
                _ => {
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
